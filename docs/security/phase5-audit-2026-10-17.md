# Phase 5 security and privacy audit - 2026-10-17

Read-only audit by three specialists in parallel (`rust-engineer`: core and
native boundary; `kotlin-specialist`: network, intents, Kotlin input;
`mobile-developer`: storage, backups, permissions), each finding checked
against the code before it was acted on. Fixes landed in #98 to #101, each
reviewed after opening (`cavecrew-reviewer`, then `/code-review`).

## What leaves the phone

| Request | Host | Carries | Trigger |
|---|---|---|---|
| RetroAchievements API | URLs built by rcheevos, https only (`RaEngine.kt`) | Username and password at sign-in; then username, token, game hash, unlocks | Sign-in, then playing a game with a set |
| Achievement badges | Any https host the RA server names | Nothing but the URL | Opening the achievements list |
| Cover art | `thumbnails.libretro.com` | Only a No-Intro name from the shipped DAT or the game-code index - never the player's file name | Cover download, if the player said yes |

Every request now sends `GBee/vX (Android N)` as User-Agent (#101). Before,
covers and badges sent the Dalvik default, with the device model and build id.
Responses are capped: RA 16 MiB (Kotlin and, since #99, C), covers 4 MiB,
badges 1 MiB. No device id is sent anywhere.

## What stays on the phone

| Asset | Where | Other apps | Backup | Logcat (release) |
|---|---|---|---|---|
| RA token | `shared_prefs/retroachievements.xml`, private | No | Excluded | Never logged |
| Battery saves, `.bak` | `files/saves` | No | Included on purpose | No path since #98 |
| Save states, `.bak` | `files/states` | No | Included on purpose | No path since #98 |
| Covers | `files/covers` | No | Excluded since #98 | - |
| Badges | `files/badges` | No | Excluded since #98 | - |
| ROM folder grants | `rom_folders` prefs + SAF | No | Prefs backed up, grants are not (dead after restore) | No Uri since #98 |

Release builds strip `Log.v/d/i` (`proguard-rules.pro`), and every
`Log.w/e` that carried a path, Uri, file name or exception message now
carries only the exception class.

## Permissions and exported components

Merged release manifest: `INTERNET`, `VIBRATE`, and androidx's
signature-level `DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`. No storage
permission. Exported: `MainActivity` (launcher only - the unused `VIEW`
filter for ROM mime types was removed in #98) and androidx's
`ProfileInstallReceiver`, guarded by `android.permission.DUMP`.

## Findings and fixes

| Finding | Fix |
|---|---|
| A v6..9 save state skips the ROM CRC check, so an imported one from another game replaced this game's battery save | Import accepts only v10..11 (#100) |
| A battery save the core refused still let later flushes overwrite the good `.sav` | Saving is turned off for that session, with a message (#100) |
| Kotlin-supplied FFI lengths could size a multi-GiB `Vec` and abort | Clamped (#100) |
| A caught panic in `gbee_save_write` reported success | Returns -1; logged once per process (#100) |
| Achievement and leaderboard lists were read outside `coreLock` while unload frees them (use-after-free) | Read under `coreLock` off the main thread (#99) |
| `nativeServerResponse` had no C-side cap and no NUL terminator | 16 MiB cap, `len+1` NUL-terminated copy (#99) |
| A badge declaring huge dimensions could exhaust memory | Bounds read first, sides capped at 512 (#101) |
| Export backups used a provider-supplied name as a path | Last path segment only (#101) |
| Covers and badges went to cloud backup; file paths reached release logs | #98 |

## Decisions

- **Old exported states.** Import refuses v6..9 states, including ones the
  app itself exported before v10: without the ROM CRC there is no way to tell
  a legitimate old state from another game's. They still load on the phone
  that made them.
- **Already safe, no change:** zip handling (entry names never used as
  paths; inflated bytes capped at 32 MiB), hostile ROMs (0 bytes to
  32 MiB + 1 rejected), truncated or lying save states (rejected and rolled
  back), every JNI byte array read by its own length, https only.
- **rcheevos** is pinned at `116d018` on its `develop` branch (rc_version
  12.5.0). Upstream code was not audited.
