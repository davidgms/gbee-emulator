# Play Console assets

| File | Play field | Size |
|---|---|---|
| `icon-512.png` | App icon | 512x512, RGBA |
| `feature-graphic.png` | Feature graphic | 1024x500, RGB, no alpha |
| `screenshots/*.png` | Phone screenshots | 1080x1920, RGB |
| `listing.md` | App name, short and full description | - |

`icon-512.png` and `feature-graphic.png` are drawn by `make.py` from the
launcher foreground (`android/app/src/main/res/drawable-nodpi/ic_launcher_foreground.png`),
the honeycomb in `ic_launcher_background.xml` and the wordmark in
`docs/logo.png`, scaled nearest-neighbour at integer factors. It needs Pillow,
which is not installed system-wide:

```
pip install --target temp/pylib pillow
PYTHONPATH=temp/pylib python3 -I docs/store/make.py
```

`listing.md` holds the app name, short description and full description to
paste into Play Console. Every feature it names was checked against the code on
2026-10-18; Bluetooth controllers are left out because pad mapping is not built
(the Settings row says "Coming soon!").

**The feature graphic is a placeholder.** Play needs one to publish, but it
only shows on a promo video cover and in featured rows, never on the app page
itself. The current one is to be redrawn before release.

## Screenshots

Seven phone screenshots, 1080x1920, one feature each, in the order the full
description follows: play, layout editor, custom buttons, button colour
themes, save states, screen filters, RetroAchievements.

`shots.py` composes them: a pixel-font headline and subline over the night
palette (`docs/design/DESIGN-SYSTEM.md`), honeycomb, sparkles and the
`loading_bee.png` bees placed as they are, around a real app capture. The raw
captures it reads (`temp/listing/s-*.png`) are not committed; they were taken
on the `gbstorage` emulator (Android 10, x86_64) with a demo-mode status bar.

The game shown is **Butano Fighter**, a free homebrew game by GValiente
(https://gvaliente.itch.io/butano-fighter). Its art is CC0 by Pixel-boy
(Sparklin Labs), credited in the full description. No commercial game, box
art or Nintendo mark may appear in a store asset, and the ROM itself is never
committed.
