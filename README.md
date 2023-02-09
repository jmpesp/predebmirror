Run this before the proper [debmirror][1] to download packages in parallel.

It's good at saturating your network card:

```
$ nicstat -i eno3 2
    Time      Int   rKB/s   wKB/s   rPk/s   wPk/s    rAvs    wAvs %Util    Sat
11:19:50     eno3  2830.9   333.2  2014.2   429.5  1439.2   794.6  2.32   0.00
11:19:52     eno3 84811.2   558.3 58082.3  8441.3  1495.2   67.72  69.5   0.00
11:19:54     eno3 90286.7   565.9 61684.4  8815.7  1498.8   65.73  74.0   0.00
11:19:56     eno3 59482.7   428.3 40747.1  6610.1  1494.8   66.35  48.7   0.00
11:19:58     eno3 51228.2   466.4 35254.1  7059.3  1488.0   67.66  42.0   0.00
11:20:00     eno3 55590.9   483.0 38191.3  7341.1  1490.5   67.37  45.5   0.00
11:20:02     eno3 53594.4   401.4 36826.8  6193.5  1490.2   66.37  43.9   0.00
11:20:04     eno3 49366.3   393.5 33850.7  6070.6  1493.4   66.38  40.4   0.00
11:20:06     eno3 66282.7   434.4 45324.7  6843.4  1497.5   65.01  54.3   0.00
11:20:08     eno3 75766.5   480.2 51822.5  7629.1  1497.1   64.45  62.1   0.00
11:20:10     eno3 84796.2   558.6 58176.3  8900.8  1492.6   64.26  69.5   0.00
11:20:12     eno3 73358.8   447.1 50282.5  7085.8  1493.9   64.62  60.1   0.00
11:20:14     eno3 68888.8   393.4 47315.0  6169.1  1490.9   65.30  56.4   0.00
11:20:16     eno3 68049.0   352.2 46700.0  5675.3  1492.1   63.54  55.7   0.00
11:20:18     eno3 74145.3   417.5 50904.7  6647.0  1491.5   64.32  60.7   0.00
11:20:20     eno3 76355.3   475.2 52502.8  7505.3  1489.2   64.83  62.6   0.00
11:20:22     eno3 71830.0   441.1 49406.0  6978.9  1488.8   64.73  58.8   0.00
11:20:24     eno3 83298.8   527.9 57224.8  8448.9  1490.6   63.98  68.2   0.00
11:20:26     eno3 91950.3   583.8 62971.6  9207.5  1495.2   64.93  75.3   0.00
11:20:28     eno3 77325.4   480.6 52899.2  7566.1  1496.8   65.04  63.3   0.00
```

It does **not** perform the same GPG checks that the real debmirror does!
Please run the real debmirror after this.

There are a lot of TODOs in here, user beware. Note that:

- Dist, component, and arch are hard-coded.
- The list of Debian mirrors is hard-coded.

Please change these for your purposes.

[1]: https://packages.debian.org/stable/debmirror
[2]: https://manpages.debian.org/bullseye/debmirror/debmirror.1.en.html

