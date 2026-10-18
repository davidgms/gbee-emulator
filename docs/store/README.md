# Play Console assets

| File | Play field | Size |
|---|---|---|
| `icon-512.png` | App icon | 512x512, RGBA |
| `feature-graphic.png` | Feature graphic | 1024x500, RGB, no alpha |
| `screenshots/*.png` | Phone screenshots | 1080x2400 |

`icon-512.png` and `feature-graphic.png` are drawn by `make.py` from the
launcher foreground (`android/app/src/main/res/drawable-nodpi/ic_launcher_foreground.png`),
the honeycomb in `ic_launcher_background.xml` and the wordmark in
`docs/logo.png`, scaled nearest-neighbour at integer factors. It needs Pillow,
which is not installed system-wide:

```
pip install --target temp/pylib pillow
PYTHONPATH=temp/pylib python3 -I docs/store/make.py
```

The screenshots were taken on the `gbstorage` emulator (Android 10, x86_64)
with demo-mode status bar, on 2026-10-18. They show only the 240p Test Suite
and the flash64 test ROM: no commercial game, box art or Nintendo mark may
appear in a store asset.
