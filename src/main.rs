use anyhow::Result;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use tokio::sync::mpsc;

// https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d
async fn download_file(
    pb: &ProgressBar,
    client: &reqwest::Client,
    name: &str,
    version: &str,
    url: &str,
    path: &str,
) -> Result<(), String> {
    let res = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    pb.set_style(ProgressStyle::default_bar()
        .template("{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").map_err(|e| e.to_string())?
        .progress_chars("#>-"));

    pb.set_length(total_size);
    pb.set_message(format!("{} ({})", name, version));

    let mut file = File::create(path).or(Err(format!("Failed to create file '{}'", path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    return Ok(());
}

fn compare_file_hash(path: &str, digest: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let contents = std::fs::read(&path)?;
    let result = sha256::digest(&contents[..]);
    Ok(result.to_lowercase() == digest.to_lowercase())
}

async fn conditional_download(
    pb: &ProgressBar,
    client: &reqwest::Client,
    name: &str,
    version: &str,
    url: &str,
    sha256: &str,
    bytes: u64,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if Path::new(&path).exists() {
        let meta = std::fs::metadata(&path)?;

        if meta.len() != bytes {
            download_file(&pb, &client, &name, &version, &url, &path).await?;
        } else {
            pb.set_style(ProgressStyle::default_bar().template("{msg}")?);
            pb.set_message(format!("checking SHA256 for {}", path));

            if !compare_file_hash(&path, &sha256)? {
                pb.set_message(format!("bad SHA256 for {}, re-download", path));
                download_file(&pb, &client, &name, &version, &url, &path).await?;
            } else {
                pb.set_message(format!("ok SHA256 for {}", path));
            }
        }
    } else {
        std::fs::create_dir_all(Path::new(&path).parent().unwrap())?;
        download_file(&pb, &client, &name, &version, &url, &path).await?;
    }

    Ok(())
}

// https://docs.rs/flate2/latest/flate2/read/struct.GzDecoder.html
fn decode_reader(bytes: Vec<u8>) -> io::Result<String> {
    let mut gz = GzDecoder::new(&bytes[..]);
    let mut s = String::new();
    gz.read_to_string(&mut s)?;
    Ok(s)
}

#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub filename: String,
    pub sha256: String,
    pub size: u64,
}

impl Package {
    fn new() -> Package {
        Self {
            name: "".to_string(),
            version: "".to_string(),
            sha256: "".to_string(),
            filename: "".to_string(),
            size: 0,
        }
    }
}

// Number of threads == number of mirrors
#[tokio::main(worker_threads = 6)]
async fn main() -> Result<()> {
    let dur = std::time::Duration::from_secs(60);
    let client = reqwest::ClientBuilder::new()
        .connect_timeout(dur)
        .timeout(dur)
        .build()?;

    let mirror_list = vec![
        // Canada mirrors
        "http://ftp.ca.debian.org/debian/",
        "http://debian.mirror.iweb.ca/debian/",
        "http://debian.mirror.rafal.ca/debian/",
        "http://mirror.csclub.uwaterloo.ca/debian/",
        "http://mirror.estone.ca/debian/",
        "http://mirror.it.ubc.ca/debian/",
    ];

    let mut mirror_tasks = Vec::with_capacity(mirror_list.len());
    let mut mirror_channel_list: Vec<mpsc::Sender<Package>> = Vec::with_capacity(mirror_list.len());
    let mb = MultiProgress::new();

    // Spawn a channel for each mirror
    for mirror in mirror_list {
        let (tx, mut rx) = mpsc::channel::<Package>(1000);

        let client = reqwest::ClientBuilder::new()
            .connect_timeout(dur)
            .timeout(dur)
            .build()?;

        let pb = mb.add(ProgressBar::new(0));

        let handle = tokio::spawn(async move {
            while let Some(package) = rx.recv().await {
                let url = format!("{}/{}", mirror, package.filename);
                if let Err(e) = conditional_download(
                    &pb,
                    &client,
                    &package.name,
                    &package.version,
                    &url,
                    &package.sha256,
                    package.size,
                    &package.filename,
                )
                .await
                {
                    eprintln!("downloading {} failed: {}", package.name, e);
                }
            }
            pb.finish_with_message(format!("mirror {} done", mirror));
        });

        mirror_tasks.push(handle);
        mirror_channel_list.push(tx);
    }

    let dists = vec![
        "buster",
        "buster-updates",
        "buster-backports",
        "bullseye",
        "bullseye-updates",
        "bullseye-backports",
    ];

    for dist in &dists {
        // TODO: store Release so it can be put into our mirror, but only after
        // all packages are downloaded. this isn't necessary if this tool is run
        // before the real debmirror.
        let url = format!("http://deb.debian.org/debian/dists/{}/Release", dist);
        mb.println(format!("downloading {}", url))?;

        let release = client.get(url).send().await?.text().await?;

        // TODO verify http://deb.debian.org/debian/dists/bullseye/Release.gpg.
        // this isn't necessary if this tool is run before the real debmirror.

        let mut sha256_rows = false;
        let mut file_name_to_sha256: HashMap<String, String> = HashMap::new();

        for line in release.split("\n") {
            if line.starts_with(" ") && sha256_rows {
                let columns: Vec<&str> = line.split(" ").filter(|x| x.len() > 0).collect();
                if columns.len() == 3 {
                    let previous =
                        file_name_to_sha256.insert(columns[2].to_string(), columns[0].to_string());
                    assert!(previous.is_none());
                }
            } else {
                // start collecting lines if listing SHA256 entries
                sha256_rows = line.starts_with("SHA256");
            }
        }

        let mut mirror_channel_index = 0;

        for component in vec!["main", "contrib", "non-free", "main/debian-installer"] {
            for arch in vec!["amd64", "i386"] {
                // TODO: store this so it can be put into our mirror, but only after
                // all packages are downloaded

                // TODO only Contents-all.gz seems to be on main mirror, try others?
                let url = format!(
                    "http://deb.debian.org/debian/dists/{}/{}/binary-{}/Packages.gz",
                    dist, component, arch,
                );
                mb.println(format!("downloading {}", url))?;

                let packages_compressed = client.get(url).send().await?;

                let packages_text = decode_reader(packages_compressed.bytes().await?.to_vec())?;

                let mut package = Package::new();

                for line in packages_text.split("\n") {
                    if line.starts_with("Package: ") {
                        // new package starting
                        if package.sha256.len() == 64 {
                            mirror_channel_list[mirror_channel_index]
                                .send(package)
                                .await?;

                            mirror_channel_index += 1;
                            if mirror_channel_index >= mirror_channel_list.len() {
                                mirror_channel_index = 0;
                            }
                        }

                        package = Package::new();
                        package.name = line.split(": ").collect::<Vec<_>>()[1].to_string();
                    }

                    if line.starts_with("Version: ") {
                        package.version = line.split(": ").collect::<Vec<_>>()[1].to_string();
                    }

                    if line.starts_with("Filename: ") {
                        package.filename = line.split(": ").collect::<Vec<_>>()[1].to_string();
                    }

                    if line.starts_with("SHA256: ") {
                        package.sha256 = line.split(": ").collect::<Vec<_>>()[1].to_string();
                    }

                    if line.starts_with("Size: ") {
                        package.size = line.split(": ").collect::<Vec<_>>()[1].parse()?;
                    }
                }
            }
        }
    }

    // drop all senders
    for mirror_channel in mirror_channel_list {
        drop(mirror_channel);
    }

    mb.println("waiting until all tasks are done")?;

    // wait until tasks are done
    for task in mirror_tasks {
        task.await?;
    }

    Ok(())
}
