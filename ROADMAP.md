# GBee Emulator - Android Roadmap

The plan, ordered by what actually blocks the next milestone.

**This file is Android only.** Every phase below - the emulation core included,
since the core exists to be played on a phone - is scoped to shipping the
Android app on Google Play. iOS has its own file,
[`ROADMAP-ios.md`](ROADMAP-ios.md), and it is **deliberately not being worked
on**. Nothing in it starts until Phase 6 here is finished and the app is live
on Play, and then only after asking the repository owner whether the iOS work
should begin at all. Do not open iOS tasks before that gate; do not treat an
idle moment on Android as a reason to start them.

**How to read this file.** A box is ticked only when something verifies it - a
test in `core/tests/`, or a measurement on a device. Code existing is not the
same as code working: on 2026-08-27 the ARM and THUMB decoders were marked
"all instructions" complete while every second instruction was being skipped
and most of the THUMB instruction set was decoding to a NOP. Unverified work is
listed as unverified. Finished phases are kept short: the decisions stay here,
the stories are in [`.claude/memory.md`](.claude/memory.md) and in the PRs
named.

---

## Current state

Line counts and test counts as of 2026-10-27 (branch with #112 and #113).

| Module | Lines | Status |
|--------|-------|--------|
| `cpu/` | ~2630 | ARM + THUMB decoders, full banking, per-access cycle accounting and the prefetch buffer (#69). Passes gba-suite `arm`/`thumb`/`memory`/`bios`, FuzzARM and ARMWrestler. 46 tests in `core/tests/cpu.rs`. |
| `ppu/` | ~1610 | Mode 0 verified against test ROMs and real games. Modes 1-5 are approximate: no OBJ-vs-BG priority, blend 2nd target by draw order, bitmap BG2 ignores its affine registers and windows. Bitmap modes honour the BG2 enable since #113. Optional interframe blending. |
| `apu/` | ~1280 | 4 PSG channels + FIFO A/B. Frame sequencer at 512 Hz and SOUNDCNT_L/H routing and volume since #112. Driven by real games: *Yggdra Union*'s DMA music on a device since 2026-08-31, *Pokemon Emerald*'s title compared against mGBA (envelope correlation 0.665 -> 0.980). Mono output, no band-limiting, and two open timing bugs (see Known accuracy gaps). |
| `memory/` + `io.rs` | ~1170 | Region mirroring, 8-bit video write rules, I/O read masks and open bus, per-region wait states from WAITCNT. |
| `dma.rs` | ~380 | 4 channels, immediate/HBlank/VBlank/sound, latch, stall. Video capture (timing 3) absent. |
| `timer/` | ~250 | Prescaler on global edges, cascade, IRQ timed from the overflow cycle. |
| `cart/` | ~900 | ROM load, save type detection, SRAM, Flash with chip ID, EEPROM serial protocol with width from DMA length. No RTC. |
| `savestate.rs` | ~920 | **Format v11**; v6-v11 load. Carries the ROM CRC-32 (a state from another ROM is refused) and the between-step live state. A rejected restore rolls back. Eight UI slots. |
| `bios.rs` | ~700 | HLE SWIs. `BgAffineSet`/`ObjAffineSet` real since #112. `BitUnPack` is still a stub; `HuffUnComp`, `Stop`, `GetBiosChecksum`, `MidiKey2Freq` and the sound driver are absent, and an unhandled SWI is a silent no-op. |
| `ffi.rs` | ~1040 | C ABI + JNI: input, frame buffer, audio, battery saves, save states as bytes, rewind, memory peek. **23** `GbaEngine_*` JNI exports. |
| `rewind.rs` | ~85 | Bounded ring of save states, wired end to end, verified on a device. |
| `android/` | ~12700 | Compose UI, JNI bridge, `AudioTrack` output as the clock, touch overlay with per-game layouts, saves and states, fast forward, rewind, achievements, 9 languages. Builds, installs and plays on a device. |
| `android/.../cpp/` | ~940 | The Kotlin to `rcheevos` bridge plus CMake for the pinned submodule. The only C in the project, on the frontend side on purpose. |
| tests | ~11700 | `core/tests/`: 19 files, ~9060 lines, 306 tests, plus 23 unit tests in `core/src` (`cargo test --release`: 331 passed, 1 ignored). `android/app/src/test/`: ~2650 lines of JVM JUnit. CI runs both plus an x86_64 emulator smoke test. Tests that need a ROM in `temp/roms/` print "skipping" and **pass** when it is missing, so in CI they are not gates. |

`ios/` is not tracked here. It is ~750 lines of SwiftUI with no Xcode project
and has never been compiled; its status lives in
[`ROADMAP-ios.md`](ROADMAP-ios.md).

**Device history, in brief.** All on a Xiaomi Mi 10T Pro (Android 12):
jsmolka's `hello.gba` 2026-08-29; all ten inputs reaching the core 2026-08-30;
*Yggdra Union* with music 2026-08-31; layouts and haptics 2026-09-14;
RetroAchievements against a real account, with *Mario Tennis: Power Tour*,
2026-09-29. The device loop is `cargo ndk` -> `gradle assembleDebug` -> the
**Windows** `adb.exe` (the WSL one cannot see the phone), with
`uiautomator dump` for widget bounds and `screencap` for the picture.

---

## Phase 0 - Make a game boot (done 2026-08-30, PR #1)

KEYINPUT initialised active-low plus `gbee_set_keys`; timer and DMA IRQs
raised; SP/LR and FIQ banks swapped; jsmolka's gba-suite wired as the gate
from `temp/roms/` (no ROM is ever committed). The suite found eleven decoder
and memory-map bugs the hand-written tests had missed. The `.so` files are
not committed; CI builds both ABIs with `cargo-ndk`.

**Exit criterion met 2026-08-30**: *Yggdra Union* reaches its title screen. Four
bugs gba-suite does not cover stood in the way: an unsigned THUMB `B` offset, a
PPU that overwrote the game's DISPSTAT, a halted CPU stepping over HBlank, and
an HLE IRQ handler acknowledging IF on the game's behalf.

## Phase 1 - Make it playable (done except the exit criterion)

All in PR #1, all with tests. Decisions worth keeping:

- **Save states are bytes** (`state_size`/`read`/`write`), not an opaque
  handle, so they can be exported. Eight slots, written atomically, keyed by
  the header's title and game code. Format history in
  [`docs/save-data.md`](docs/save-data.md).
- **The cart lives in the bus**, with the 8-bit save databus, Flash erase
  unlock and chip ID, and EEPROM as the real serial protocol.
- **Battery saves**: debounced flush off the core's dirty flag, plus a flush on
  pause, stop and `onCleared`. Since #78 they live only in app storage.
- **The APU decodes whole 16-bit registers**; per-byte decoding was where its
  bugs lived (no channel could start).
- **The core emits exactly 48 kHz** from a fractional sample clock, which got
  the fast audio path (`AUDIO_OUTPUT_FLAG_FAST successful`). It exposed PSG
  phases clocked by output samples, 64x flat since the start.

**Exit criterion:** a full game playable start to finish, with sound, on a
device, without losing progress. Everything is done except a person playing a
game to its end.

## September accuracy pass (done)

Four bugs found by playing, each with a test: the scanline drawn at HBlank
start so HBlank-handler writes land on the next line (#5), the EEPROM ready
poll answered for CPU reads (#6), interframe blending as the LCD ghosts (#7),
and sprite-over-sprite decided by priority first, OAM index only on ties -
GBATEK is wrong here (#19).

## Phase 2 - Quality

- [x] **Touch overlay** - size (shrink only, 0.7-1.0x: growing pushed controls
      off screen), opacity, drag-anywhere layouts per game and per
      orientation, custom combo/sequence/hold buttons, haptics. Device-verified
      2026-09-14.
- [x] **Screen scaling** - Fit, Integer (default) and Stretch, with nearest
      neighbour filtering.
- [~] **Screen filters** - None, Smooth, Scanlines and 2xSaI (JVM-tested; its
      cost measured in Phase 4). CRT beyond scanlines wants a real shader.
- [x] **ROM library** - cover art beside the ROM or downloaded (off by default,
      matched by game code then file name, never bundled; see
      [`docs/cover-art.md`](docs/cover-art.md)), favourites, info sheet,
      placeholder tiles, ZIP ROMs (entries sorted so a two-cart zip always
      picks the same one). The homebrew downloader was removed in #90.
- [x] **Orientation** - Automatic / Portrait / Landscape, locking the activity
      only. Landscape got L, R, Start and Select in #95.
- [~] **Audio latency** - buffer cut from 80 ms to 33 ms with the fast path
      kept. The full touch-to-photon figure is unmeasured; it needs a
      high-speed camera or a loopback. Runahead not considered.
- [x] **Fast forward on the audio clock** - 2x, 4x, 8x and Unlimited (#9, #10).
      Decision and measurements in
      [`docs/research/fast-forward.md`](docs/research/fast-forward.md).
      **Never quote one game's speed as the emulator's speed.**
- [x] **In-game furniture, reset, FPS and underrun counter, volume** - reset
      keeps the battery save and leaves no button held; "resume from quick
      save" is off by default; underruns matter more than fps, because audio
      is the clock.

## Phase 3 - Advanced

- [x] **Rewind** - a snapshot every 30 frames, depth 20 (~10 s, ~10 MB);
      loading a state clears the ring.
- [ ] Cheat codes (GameShark / CodeBreaker).
- [ ] Link cable over local WiFi.
- [ ] JIT, ARM host only, and only after the interpreter is correct.
- [~] **Screenshots** at native 240x160 into `Pictures/GBee`. Recording undone.
- [ ] Debug tools: breakpoints, memory viewer, register inspector.
- [x] **RetroAchievements, softcore** - `rcheevos` is a pinned submodule on the
      Android side, **never in `core/`**; the core gained only
      `gbee_peek_memory`. Badges in their real locked/earned state,
      leaderboards with a live tracker. See
      [`docs/achievements.md`](docs/achievements.md).
- [ ] **Hardcore achievements** - blocked by a calendar: approval and six
      months of public availability, and it forbids save states, rewind and
      frame advance.
- [ ] **A leaderboard actually exercised** - *Mario Tennis* has none, so only
      the hidden-section path is verified.

---

## Phase 4 - Review, test, and measure on a real phone

- [x] **Whole-codebase code review**, every finding verified before acting:
      agent reviews reported confident bugs that were not there, and the
      verification is the part that matters.
      Fixed in #43 to #48. Of what it left open: homebrew downloads got a hash
      and size cap (#77) and then went with the downloader (#90); wait states
      landed in #69; APU volume and routing in #112; the iOS bridge is out of
      scope. Still open: Mode 5 (see Known accuracy gaps), and
      `gbee_frame_buffer_ptr`, valid only until the next frame (documented in
      `ffi.rs`; only the parked iOS bridge uses it).
- [x] **Tests for what the review found untested.**
- [ ] **UI and UX stress test on a phone** - partly done, see
      [`docs/phase4-device-run-2026-10-01.md`](docs/phase4-device-run-2026-10-01.md):
      rapid input, save/load spam, backgrounding, ROM switching, a dropped
      network, the layout editor, 43 rotations (2026-10-16) and low storage
      (2026-10-17, which found the silent failed battery save fixed in #96).
- [ ] **Performance on a phone** - partly done, see
      [`docs/phase4-perf-run-2026-10-17.md`](docs/phase4-perf-run-2026-10-17.md):
      30 minutes with no crash, flat memory, thermal status 0, one 7 ms
      underrun. MIUI holds the panel at 50 Hz whatever the app votes. Not yet:
      battery drain (needs the cable out) and achievements cost.

**Exit criterion:** every review finding fixed or recorded as a decision, and
the app holds its frame rate and audio over a long session on a phone.

## Phase 4.5 - Learn from other emulators (done)

Eleven emulators read; conclusions in
[`docs/research/emulators-research.md`](docs/research/emulators-research.md).
No emulator has a MIUI workaround, and audio as the master clock is right.
Frontend actions: never block emulation on the draw, `appCategory="game"`,
display mode re-applied on resume, low-rate tip on Xiaomi (#53, #55);
audio-driven frameskip, watchdog and Performance Hint (#54); input read four
times a frame (#56). Then the mGBA suite, now a hard gate
(`mgba_suite_does_not_regress`, per-sub-suite floors, #71):

| Sub-suite | Before | After | PR |
|---|---|---|---|
| Timer IRQ | 0/90 | 90/90 | #60 |
| I/O read | 10/130 | 130/130 | #60 |
| Timer count-up | 729/936 | 936/936 | #64 |
| DMA | 1056/1244 | 1244/1244 | #65 |
| Memory | 1346/1552 | 1552/1552 | #65, #67 |
| BIOS math | 305/615 | 615/615 | #67 |
| Multiply long | 52/72 | 72/72 | #67 |
| SIO R/W, SIO timing | ~0 | 90/90, 4/4 | #68 |
| Timing | 236/2020 | 1880/2020 | #69 |
| **Total** | **3602/6998** | **6852/6998** | |

Declined: Timing's last 140 cells (they time the real BIOS's math, which the
HLE does not run). Deferred: Misc 6/12 (needs DMA inside an instruction,
cycle-exact ROM timing, and an unexplained 3-cycle Halt delta). #60 cost ~10%
per instruction, won back in #62. Save states carry no BIOS, so **the HLE BIOS
stub layout is part of the save-state format** (an old state returned into a
moved stub, #60).

## Phase 5 - Security and privacy (done 2026-10-17)

Network, on-device storage, permissions, hostile input, the native boundary
and exported components, all audited. Reasoning in
[`docs/security/phase5-audit-2026-10-17.md`](docs/security/phase5-audit-2026-10-17.md),
fixes in #98 to #101.

## Phase 6 - Ship it on Google Play

Owned by `mobile-app-developer`, with `visual-asset-generator` on artwork.

- [x] **Icon and store assets** - in [`docs/store/`](docs/store/README.md).
- [x] **Release signing** - upload key outside the repository, four `GBA_*`
      repository secrets. A lost key is reset from Play Console, App
      integrity.
- [x] **A signed App Bundle with both ABIs** - `libgbee_core.so` and
      `libgbee_ra.so` for `arm64-v8a` and `armeabi-v7a`, checked in CI on every
      push (#104).
- [x] **R8 rules that keep JNI** - all 23 `GbaEngine_*` and 18 `RaEngine_*`
      exports survive a release build, verified on the phone.
- [ ] **Play Console listing** - store title **GBee - GBA Emulator**, package
      `com.gbee.emulator`. Data safety: the app collects nothing; it reaches
      the network only for RetroAchievements after sign-in and for cover art
      when enabled. Answer the form from the Phase 5 audit.
- [x] **No ROMs, ever** - the AAB holds no ROM and the code links to no ROM
      site. Play removes emulators over exactly this.
- [ ] **Internal testing track first**, on a device that is not the
      development phone, then production.

**Exit criterion:** a stranger can install GBee Emulator from Google Play, point
it at their own legally dumped ROM, and play it.

### The iOS gate

When that criterion is met, and **not before**, ask the repository owner
whether to start [`ROADMAP-ios.md`](ROADMAP-ios.md). It needs a Mac, an Apple
Developer account and a second frontend kept at parity forever, so it is their
decision, not something to drift into because Android went quiet.

---

## Known accuracy gaps

**The full audit, by bug class, is
[`docs/research/gba-bug-classes.md`](docs/research/gba-bug-classes.md).** The
lesson behind this section's shape: every CPU, bus, timer and DMA bug since
2026-10-02 was caught by a test suite, and every PPU, APU and HLE BIOS bug was
found by a person playing a game. The affine-set stubs sat in an unowned list
for two months until *Pokemon Emerald* hit them. So each item now has an owner,
a priority and the games it breaks.

Priority: **P1** breaks many games or the headline game; **P2** visibly wrong in
a class of games; **P3** rare or subtle. All are `core/` work, owned by
`rust-engineer`. A fix starts with the failing test.

| # | Gap | P | Games affected |
|---|---|---|---|
| 1 | **FIFO samples lost while halted.** The halted step runs `apu.tick` before `post_tick` pops the FIFO (`lib.rs`, halted branch), and steps reach 960 cycles, so several overflows collapse into one: 197 of 401 pops a frame audible at 24 kHz. | P1 | Nearly every commercial game's DMA music (grit, aliasing) |
| 2 | **A PSG frequency-byte write retriggers the note.** The trigger bit stays latched and every byte write re-decodes the whole register (`Gba::apu_sound_write`). | P1 | Every m4a game's PSG vibrato and pitch bends, *Emerald* included |
| 3 | **No RTC** (no GPIO at `0x080000C4`). | P1 | *Pokemon Ruby/Sapphire/Emerald* (dead battery, no clock events), Boktai |
| 4 | **`BitUnPack` (SWI 0x10) is a stub**: copies bytes until a zero, ignores the unpack parameters. | P1 | Games that expand 1bpp fonts or icons |
| 5 | **Modes 1-5 have no per-pixel priority sort**: sprites always over BGs, the blend 2nd target is draw order, semi-transparent OBJs ignored. Fix together, as mode 0's sort (`ppu/mod.rs`, the `ponytail:` note). | P2 | Mode-1/2 overworlds and battle screens, affine-BG games (*Madden 06/07*) |
| 6 | **Affine OBJ texels truncate toward zero** where hardware floors. | P2 | Every scaled or rotated sprite (*Emerald* intro, evolution) |
| 7 | **Bitmap BG2 ignores PA-PD and BG2X/Y**; mode 5 paints black outside 160x128. | P2 | Mode-5 FMV and scaled video, mode 3/4 scroll and shake |
| 8 | **Ch1 sweep with shift 0 writes freq + freq back.** | P2 | Sweep sound effects (jump, laser, coin) |
| 9 | **`RegisterRamReset` clears all IWRAM** (GBATEK keeps the last 0x200 bytes) and ignores bits 5-7. | P2 | Callers that pushed LR before `swi 1`; sound state survives a reset |
| 10 | **`HuffUnComp` absent, and an unhandled SWI is a silent no-op** with no log (`cpu/mod.rs` drops `handle_swi`'s result). | P2 | Huffman-compressed assets come out blank |
| 11 | **Windows do not mask BG2 in modes 3-5.** | P3 | Windowed cutscenes over a bitmap |
| 12 | **Envelope reloaded on any SOUNDxCNT_H write**, not only on trigger. | P3 | PSG notes whose duty changes mid-note |
| 13 | **Mono output, point-sampled resampling** (1.5-2x mGBA's energy above 6.4 kHz). Stereo needs an FFI change. | P3 | Panning lost everywhere; harsh high notes |
| 14 | **`MidiKey2Freq` (0x1F) and `GetBiosChecksum` (0x0D) absent.** | P3 | m4a builds that call the BIOS for pitch; anti-tamper checks |
| 15 | **SOUNDCNT_X bits 0-3 (channel active) not modelled.** | P2 | Engines that poll them cut notes (*Minish Cap* unlock jingle) |
| 16 | **FIFO refill polled at <=16 bytes, not on request; master-off does not reset it; overflow unmodelled.** | P2 | *Beyblade V-Force*, *Pac-Man World*, *Super Robot Wars J* |
| 17 | **Affine BG reference reloads only when BGxX/Y changes value**, so HBlank tables rewriting the same value keep accumulating. | P3 | *Madden 06/07* |
| 18 | **Flash byte-program overwrites instead of clearing bits only**; no save-type override list. | P3 | Games that rely on program-AND; mis-detected saves (*Tactics Ogre*) |

Accepted simplifications, recorded so they are not rediscovered as bugs:

- Instructions are atomic: a DMA lands at the end of the instruction it
  started in, a RAM-only DMA does not let the prefetch buffer fill, and a
  DMA's IF is set at transfer end. HALT wakes on the next PPU, timer or serial
  event rather than an arbitrary cycle.
- Affine PB/PD accumulate once per scanline, not mid-line.
- No BIOS execute-permission checks; no VRAM/OAM contention cycle.
- Channel 3's bank bit is fixed for a 64-digit sweep, as mGBA does; GBATEK is
  silent.
- PPU and APU *timing* have no test-ROM coverage. The ROM-based PPU tests
  check what comes out, not when.

---

## Who owns what

| Area | Agent |
|------|-------|
| `core/` - emulation, decoders, FFI | `rust-engineer` |
| Android frontend | `kotlin-specialist` |
| iOS frontend | `swift-expert` - **parked**, see [`ROADMAP-ios.md`](ROADMAP-ios.md) |
| Storage, lifecycle, battery, permissions across both platforms | `mobile-developer` |
| Build, release, device testing, platform parity | `mobile-app-developer` |
| GBA hardware questions, GBATEK, reference emulators | `search-specialist` |
| Touch targets, contrast, screen readers | `accessibility-tester` |
| Unit tests in `core/tests/` and `app/src/test/`, macro tests through `run_frame`, gba-suite harnesses | `test-automator` |
| Icons, theme art, store assets | `visual-asset-generator` |
| Multi-lane decomposition and cross-specialist consultation | `agent-organizer` (second in command; its plans are reviewed before they run) |

---

## References

| Resource | Link |
|----------|------|
| GBATEK (hardware reference) | https://problemkaputt.de/gbatek.htm |
| TONC (GBA programming) | https://www.coranac.com/tonc/text/toc.htm |
| ARM7TDMI TRM | https://developer.arm.com/documentation/ddi0029/ |
| gba-suite (test ROMs) | https://github.com/jsmolka/gba-suite |
| mGBA (reference implementation) | https://github.com/mgba-emu/mgba |
| SkyEmu (per-pixel PPU) | https://github.com/skylersaleh/SkyEmu |
| NanoBoyAdvance (cycle accuracy) | https://github.com/nba-emu/NanoBoyAdvance |
