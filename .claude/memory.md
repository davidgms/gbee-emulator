# Project Memory - GBee Emulator

## Purpose

Project context, discoveries and learnings that are durable and shared. Agents
read this at the start of a task and correct it when it goes stale. Knowledge
that belongs to one agent's craft goes in that agent's own memory instead
(`.claude/agent-memory/<name>/`, which only that agent can read); knowledge
about the project goes here or in `docs/`, where everyone gets it.

Outdated or incorrect entries should be removed, not annotated. A wrong line
here costs more than a missing one.

## Project Scope

Game Boy Advance emulator with a pixel bee theme, shipping on Android.

- **`core/`** - Rust emulation core (~11.7k lines). ARM7TDMI interpreter, PPU,
  APU, memory bus, DMA, timers, cartridge saves, save states, HLE BIOS.
- **`android/`** - Kotlin + Jetpack Compose. The only target being worked on.
- **`ios/`** - Swift + SwiftUI. Out of scope until the app is live on Play
  (`ROADMAP.md`, top), and not buildable.
- Licence MIT. No ROMs or BIOS images in the repository, ever.

## Tech Stack

- **Core**: Rust 2021, dependencies limited to `log` and `thiserror`
  (plus `jni` and `android_logger` under `cfg(target_os = "android")`).
- **Android**: Kotlin, Jetpack Compose, minSdk 26, targetSdk 36, NDK 27,
  ABIs `arm64-v8a` and `armeabi-v7a`. `android/app/build.gradle.kts` is the
  source of truth for these numbers, not the README.
- **Toolchain**: Docker image with the full Android SDK/NDK, or local
  `rustup` + `cargo-ndk`. Development happens on WSL2.

## Key Files

- `core/src/lib.rs` - `Gba::run_frame`/`step`, the main loop, DMA sound
  routing, interrupt delivery, the halted path.
- `core/src/cpu/mod.rs` - `Cpu::step`, PC advance, banked registers, IRQ entry.
- `core/src/cpu/arm.rs` / `thumb.rs` - the two instruction decoders.
- `core/src/ffi.rs` - the only boundary between core and frontends: C ABI plus
  JNI exports.
- `core/src/io.rs` - I/O register constants, read rules, interrupt state.
- `core/tests/` - the accuracy gate. See CLAUDE.md, Testing.
- `android/.../engine/GbaEngine.kt` - Kotlin side of the JNI bridge.
- `android/.../engine/AudioOutput.kt` - `AudioTrack` sink; the timing master.
- `android/.../viewmodel/EmulationViewModel.kt` - the emulation loop driver.
- `ROADMAP.md` - what is actually done and what is next, and the owned list of
  known accuracy gaps. Trust it over the feature checklists in `README.md`.
- `docs/research/gba-bug-classes.md` - the accuracy audit by bug class.

## Hardware references

Answer hardware questions from these rather than from memory:

- GBATEK - https://problemkaputt.de/gbatek.htm - the primary reference.
- TONC - https://www.coranac.com/tonc/text/toc.htm - the programmer's view.
- ARM7TDMI TRM - https://developer.arm.com/documentation/ddi0029/
- mGBA, SkyEmu, NanoBoyAdvance - reference implementations worth reading when
  a behaviour is ambiguous. **Read them for behaviour, never copy or translate
  their code**: GBee is MIT, mGBA is MPL-2.0 (file-level copyleft) and
  NanoBoyAdvance GPL-3.0. Derive code from GBATEK and hardware test tables;
  port only permissively licensed originals, with their notice in the source
  and in `THIRD_PARTY_NOTICES.md`. On 2026-10-04 the multiply-carry model
  (ported from NBA) and the HLE Div/ArcTan/ArcTan2 (shaped after mGBA's
  `bios.c`) were both rewritten for this reason before merge.

**Fetching GBATEK**: the combined `gbatek.htm` truncates badly through
WebFetch. Use the split pages, `https://problemkaputt.de/gbatek-<slug>.htm`
(for example `gbatek-arm-cpu-exceptions.htm`, `gbatek-gba-timers.htm`), and
find a slug with a `site:problemkaputt.de` search. The ARM TRM redirects to a
JS-rendered page that cannot be fetched; GBATEK's *ARM CPU Exceptions*
reproduces its exception-entry table.

## Discoveries

### 2026-08-27/28 - The generated core was broken, and the decoders taught the rules

**Context**: the core arrived "complete" on 2026-08-25. The first hand-written
test suite found four fatal decoder bugs in minutes (PC advanced by 8 so every
second instruction was skipped; THUMB dispatch matched 3 bits against 4- and
5-bit arms so most of THUMB was a NOP; ARM halfword transfers absent; MRS/MSR
selected on bit 4 instead of 21). jsmolka's gba-suite then found eleven more
(inverted `I`/`U` bits, dispatch ranges that excluded `SWPB`/`SMULL`/register
shifts, missing `P` bit and `IB`/`DA`, empty list, base-in-list, user-bank `^`,
`Rd == Rn` writeback, flags written on non-S ops, a memory map with no
mirroring). Alongside: KEYINPUT read 0 (all ten buttons held) because
`io_regs` is zero-filled and the register is active-low; ARM-mode `swi` passed
the whole 24-bit comment where the BIOS reads bits 23-16 (`ldrb [lr,#-2]`), so
every ARM-state BIOS call did nothing while THUMB worked; and LZ77/RL looped
forever because they never read the header's size (bits 8-31).

**Application**:

- Rust does not warn about an unreachable integer match arm, so a dispatch
  table rots silently. Add the encoding to `core/tests/cpu.rs` before
  touching a decoder, and run `cargo test --release --test gba_suite`.
- **A round-trip test that does not check the address proves nothing.**
  `arm_ldr_str_word` passed while store and load used the same wrong address.
- Treat any `unreachable!()` in a decode path as a latent panic.
- Completion claims in `ROADMAP.md` and `README.md` are unverified until a
  test covers them.

### 2026-08-27 - Ordering trap in the ARM dispatch chain

`halfword_data_transfer` must be tested **before** the PSR transfer check. A
pre-indexed, down-counting `STRH` carries the same bits `[24:23] = 10` and
`bit [20] = 0` pattern that `MSR` matches on, so putting it later silently
routes stores into the PSR path. The dispatch chain in `core/src/cpu/arm.rs`
is order-dependent throughout; adding an arm means checking what already
claims that bit pattern.

### 2026-08-28 - write16 and write32 decompose into byte writes

`MemoryBus::write16`/`write32` are implemented as repeated 8-bit stores. That
made adding the GBA's 8-bit video rules (OAM ignores byte writes; palette and
BG VRAM duplicate the byte across the halfword; OBJ VRAM ignores them)
dangerous, because those rules must apply **only** to genuine byte stores.
The fix was a private `store8` that the wider writes use, with the public
`write8` layering the video rules on top. If you add another size-dependent
rule to the bus, check which of the two paths it belongs on.

### 2026-08-28 - Grep for a caller before believing a feature is done

The cart's save memory (`Cartridge::save_read`/`save_write`/`save_data`/
`load_save`) was complete and had **zero callers**: `MemoryBus` had no
`Cartridge` and no arm for `0x0E000000`, so every in-game save was discarded.
The same shape repeated six more times in the next week - DMA register setters
(2026-08-30), timer setters, the empty `on_timer_overflow` stub,
`apply_alpha_blend` (2026-08-31), rewind with no FFI, and the jniLibs `.so`
that docs called "committed" but `.gitignore`'s `*.so` rule had always
excluded. A number repeated from prose has the same failure: README's minSdk
24 was copied into CLAUDE.md, this file and two personas while
`build.gradle.kts` said 26.

**Application**: when a feature is listed as done, grep for a *caller*, not a
`pub fn`. Treat a function whose body is a comment as a missing feature, not a
simplification - `on_timer_overflow` documented the hardware behaviour it did
not implement. When a layer boundary exists (core/FFI/frontend), check both
sides of it. Read a number from the file that defines it.

### 2026-08-28 - `cargo check --target aarch64-linux-android` compiles the JNI block

The JNI exports in `core/src/ffi.rs` sit behind `#[cfg(target_os = "android")]`,
so a normal `cargo build`/`cargo test` never type-checks them - broken JNI code
passes the whole local gate and only fails in CI or on a device. `cargo check
--target aarch64-linux-android` compiles that block **without needing the
NDK**. It caught two real errors on its first use (`JNIEnv` must be `mut env`
for `new_byte_array` and `get_array_elements`). Run it after touching `ffi.rs`.

### 2026-08-28 - Mode 0 never read tile data: a VRAM offset used as an address

`Ppu::get_bg_pixel` passed `char_base` - a VRAM *offset* - straight to
`bus.read8()`, so every tile read landed around `0x4000` instead of
`0x06004000` and decoded to colour 0. It was invisible because
`render_scanline` pre-filled each line with **white** rather than the
backdrop. Two bugs hiding each other. Tile flip bits and the 4bpp palette bank
were ignored too.

**Application**: in `ppu/`, be explicit about whether a value is a VRAM offset
or a bus address - the two differ by `0x06000000` and only one faults visibly.
Never clear a scanline to a colour that could be mistaken for real output.

### 2026-08-28 - The APU register map was off by one byte, so no channel started

`write_sound1_reg` decoded `SOUND1CNT_H` as if the whole register were in the
low byte, treated `0x64` as a high byte, and had **no handler for `0x65`**,
where the trigger bit sits. `Gba::apu_sound_write` also routed only about half
the sound registers. `SOUNDCNT_X` (master enable) was not routed at all, and
`sound_on` was read from SOUNDCNT_H bit 15, which is "DMA Sound B Reset FIFO".
Fixed by decoding at 16-bit register granularity and folding every captured
byte onto its containing register.

**Application**: byte-level register decoding is where this class of bug
breeds. Decode whole registers, and ask for output - a 16-bit layout flattened
into the low byte looks plausible until something has to sound.

### 2026-08-30 - A press shorter than one frame used to be dropped

`EmulationViewModel` samples `keyState` once per frame. A synthetic `adb input
tap` presses for 3 ms, so the loop never saw it. Fixed with a
`transientPresses` AtomicInteger that latches every press until the loop
drains it.

**Application**: when a consumer polls state on a fixed cadence, decide
explicitly what happens to events shorter than the period.

### 2026-08-30 - Debugging on device: trust the log over the screen

A single sampled pixel kept landing in the letterbox; one `log::warn!` in the
JNI function settled it (`nativeSetKeys keys=0x8 -> KEYINPUT=0x03F7`). Two
traps: a hand-written test ROM is itself unverified code (**halfword loads
carry only an 8-bit immediate offset**, so `[r0, #0x130]` silently encoded as
`[r0, #0x10]`), and `android_logger` is capped at `LevelFilter::Warn`, so
`log::info!`/`debug!` from Rust never reach logcat. MIUI's `ACDB-LOADER ...
set parameters failed` lines are present for any app, not ours.

### 2026-08-30 - DMA was never wired to the bus

The bus stored DMA registers into `io_regs` and `Dma::channels[]` never heard
about them; Yggdra hung polling DMA3CNT_H bit 15. `MemoryBus::store8` now
queues the channel whose DMAxCNT_H high byte was written (`dma_writes`), and
`Gba::step` drains it through `apply_dma_writes`. The bus cannot apply it
itself: `write_control` runs an immediate transfer inline and needs `&mut
MemoryBus`, the borrow the store already holds. Same shape as `sound_writes`
and `timer_writes`.

Making DMA live exposed three bugs in code that had never run: source and
destination address-control fields read backwards (bits 5-6 are **dest**),
"0 means maximum length" truncated back to 0 by `as u16`, and repeat reloading
the *source*, which flattens an HDMA table. Still not implemented: video
capture DMA (timing 3) is an empty loop in `Dma::on_vcounter`.

### 2026-08-30 - Yggdra Union boots: four bugs gba-suite does not cover

1. **THUMB `B label` never sign-extended its 11-bit offset** (`as i16` of
   `& 0x7FF` is always positive), so every backward branch went 0x1000 up.
   jsmolka's `thumb.gba` passes with the bug present.
2. **The PPU rebuilt DISPSTAT every tick from cached enable bits**, erasing a
   mid-scanline write; bit 2 was hard-wired to VBlank and the VCount IRQ never
   fired. Bits 3-15 are the game's and are now preserved.
3. **A halted step advanced a whole scanline**, stepping over HBlank: one IRQ
   a frame instead of 228. `Ppu::cycles_to_next_event` bounds the step.
4. **The HLE BIOS IRQ handler acknowledged IF** before calling the game's
   handler. The real one only saves r0-r3/r12/lr, jumps to `[0x03007FFC]` and
   returns; IF and the IntrWait flags at `0x03007FF8` are the game's job. HLE
   `IntrWait` now polls `[0x03007FF8]` and re-executes its own SWI.

**Application**: two of four were our HLE doing the game's work for it. When
an HLE stub is "helpful", check GBATEK for what the real BIOS does.

### 2026-08-30 - A reference emulator is worth more than any amount of tracing

Three of the four Yggdra bugs took hours of hand-tracing; the fourth took ten
minutes in mGBA (a write watchpoint on DISPCNT, then the caller chain, then
`dis/a 0x03000000`). `mgba-sdl` installs **without root**: `apt-get download`
the package and its dependencies, `dpkg-deb -x` into one tree, run with
`LD_LIBRARY_PATH`. `temp/mgba/run.sh` does this. Headless:
`SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy temp/mgba/run.sh -d <rom>`, then
feed the CLI debugger on stdin (`watch/w`, `b`, `c`, `i`, `dis/t`, `dis/a`,
`trace N`).

**Application**: reach for it *first* on "game X does not boot". mGBA boots
through its own BIOS HLE from `0x00000350`, so compare behaviour at named
addresses, not step numbers.

### 2026-08-31 - Timers unwired, FIFO stub, blend stub: the thump

Yggdra's "tum tum" audio and grey bar were three faults:

1. **Timer setters had no caller**, so no game timer ran: no timer IRQ, no DMA
   sound. Fixed with a `timer_writes` queue latched on each register's high
   byte. `TMxCNT_L` now reads the live counter.
2. **`Apu::on_timer_overflow` was empty** and the mixer popped the FIFOs once
   per output sample, so sound played at our rate and the source ran off the
   game's buffer into IWRAM - a burst of garbage every six frames. Now a timer
   overflow pops one byte into a latch, the mixer holds the latch, the DMA tops
   up at half empty. `overflow_flags` is a count, since one tick can span
   several overflows. DMA sound volumes are SOUNDCNT_**H** bits 2-3, and each
   FIFO picks its own timer (H bit 10 for A, 14 for B).
3. **`apply_alpha_blend` was empty**, and effects were gated on `bldcnt & 0x20`
   (the *backdrop* 1st-target bit). Mode 0 now composites the top two layers
   per pixel.

Also: `Gba::step`'s halted branch skipped FIFO refill, timer overflows, queued
sound writes and DMA application. A game is halted most of every frame, so
that block is now a shared `post_tick` run on both paths.

### 2026-08-31 - Sprites had never rendered correctly

Four faults in `render_obj_scanline`: OBJ tiles read from `0x06000000`
(they live at `0x06010000`, `0x06014000` in bitmap modes), so every sprite was
textured with BG tiles; the depth bit read from attr0 bit 7 (part of Y) instead
of bit 13; 8bpp tiles stepped by 64 instead of two 32-byte units (2D mapping is
a fixed 32-unit grid); and X/Y sign-extended when they are unsigned and wrap.
Flips were missing and OAM order was backwards. The affine path added the OAM
base twice and read its matrix from the save chip.

**Application**: gba-suite, three homebrew ROMs and 40,000 frames of Yggdra's
attract loop were all background-only. "The screens I checked render
correctly" is not evidence that a feature works; reproduce the sequence the
report names. Method: render with each of BG0-3 and OBJ forced off in DISPCNT
(re-forced every step, since games rewrite it in VBlank) and diff the frames.

### 2026-08-31 - Homebrew test ROMs, and the VRAM decompressors they caught

Two freely distributable ROMs live in `temp/roms/`, with fetch commands in
`core/tests/ppu.rs`'s header: **Celeste Classic** (sprite-heavy) and the
**240p Test Suite** (`240pee_mb.gba`, a purpose-built PPU suite whose menus are
LZ77 into VRAM). The 240p suite found that `LZ77UnCompVram`/`RLUnCompVram`
wrote bytes, and VRAM turns a byte store into two copies across the halfword -
the reason the Vram variants exist. Both now build the block in a `Vec` and
write halfwords anywhere in `0x05000000-0x07FFFFFF` (`write_block`).

### 2026-09-01 - Sprite and background priority rules

GBATEK, *LCD OBJ - OAM Attributes*: **a sprite wins a priority tie with a
background**, and **between backgrounds the lower BG number wins a tie**. The
sprite pass records `(colour, priority)` per pixel into `Ppu::obj_pixel` and
mode 0's compositor merges it. Sprite-over-sprite order is in the 2026-09-10
entry. Semi-transparent sprites (OBJ mode 1) are always a 1st target and
always alpha-blend regardless of BLDCNT bits 4 and 6-7, but the 2nd-target
bits 8-13 decide what they blend with, and a non-target underneath means no
blend. OBJ-window sprites (mode 2) must still be decoded for their mask shape.

**Application**: ask `search-specialist` for the rules before writing the
code; two of these are coin-flip cases that look plausible either way.

### 2026-09-01 - Affine backgrounds, and what Yggdra's gameplay exercises

A scripted run (`temp/probes/play.rs`, which grew `SNAP=<frame>:<file>` and
`LOAD=<file>`) reached Yggdra's first scene in ~16,000 frames: **BG mode 1**.
`render_affine_bg_pixel` was wrong four ways: it never read the tilemap (map
index used as the tile number), skipped the layer unless BGxCNT bit 7 was set
(affine BGs are always 256-colour), always wrapped (bit 13 clear means
transparent outside), and reloaded the reference point every line, throwing
PB/PD away. The reference is now loaded at frame start, advanced by PB/PD per
line and reloaded on a mid-frame write. BGxX/Y are 28-bit signed (19.8).

Every Yggdra screen reached after that rendered correctly, including the
battle map. Yggdra never enables a window. `A` doing nothing on CARD SELECT is
game logic, not a stuck input.

**Application**: reaching real gameplay is worth more than reasoning about
which gap to close next; an opening and an attract loop can exercise a
different quarter of the PPU than the game does.

### 2026-09-01 - Moving to 48 kHz exposed a PSG that had been 64x flat all along

The core emitted at `16777216 / 964` = 17403 Hz, which no device supports, so
AudioTrack resampled and denied `AUDIO_OUTPUT_FLAG_FAST`. The sample clock is
now an exact fraction (`sample_accum += cycles * 48000`, a sample each time it
reaches 16777216). That exposed the real bug: **every PSG channel advanced its
phase once per emitted sample**, so the tone ROM's 128 Hz played at 2 Hz.
Phases now run off the system clock at GBATEK's divisors (`16*(2048-n)` per
duty step for ch1/2, `8*(2048-n)` per wave sample, `32*r*2^(s+1)` per LFSR
step with r=0 meaning 0.5).

**Application**: "audio works" had meant "samples are non-zero". Where the ROM
states a number and the output can be measured against it, assert on the
number.

### 2026-09-01 - Windows, and a "display off" bit that does not exist

Windows are a per-scanline `window: [u8; 240]` of WININ/WINOUT's low six bits,
consulted by every compositor; WIN0, then WIN1, then the OBJ window, then
outside. **WINxH holds X1, the left edge, in bits 8-15** (same for V).
OBJ-window sprites are decoded before the mask is built. And **`display_off`
was read from DISPCNT bit 15, the OBJ window enable** - there is no display-off
bit (forced blank is bit 7) - so any game enabling the OBJ window went black.
That was `fantasy-knight.gba`'s black screen, stacked on the affine and window
gaps; fixing the first two changed nothing visible.

**Application**: check what a register bit *is* before trusting a field named
after what someone assumed. When a fix changes nothing visible, look for a
second cause stacked under it.

### 2026-09-01 - Frontend: integer scale, wake lock, audio buffer

`EmulationScreen.kt` had no `keepScreenOn`. Portrait padding of 24.dp left
948 px of 1080, which integer scaling rounds to 3x; 8.dp clears 960 px, 4x.
`AudioOutput` clamped its buffer up to `getMinBufferSize` (3844 frames, 80 ms -
the safe size for the *normal* mixer); two emulated frames (1600, 33 ms) keep
the fast path with zero underruns, and since audio is the clock that buffer is
the floor on input latency. `AudioOutput.write` returns `false` for a
zero-sample write so the caller paces itself.

### 2026-09-02 - JVM unit tests for Android, and two defects tests cannot catch

`android/app/src/test/` exists since `Sai2xTest` (`gradle testDebugUnitTest`).
Put frontend arithmetic in plain functions over plain arrays and test it on
the JVM: seconds instead of a device round trip.

A self-review found two things no test would flag because neither changes a
result: `render_mode0_scanline` allocated a `Vec` per pixel (2.3M allocations
a second, now a fixed array), and the BIOS decompressors trusted the header's
24-bit size (one SWI could allocate 16 MB; clamped to 256 KB). Read hot loops
for allocations and every length from ROM data as hostile.

### 2026-09-03 - Modes 1-5's colour effects were gated on the wrong bit

Yggdra's battle screen strobed between normal and washed out. The mode 1-5
effect call was gated on `bldcnt & 0x20` (the backdrop 1st-target bit), and
brighten/darken hit every pixel of the line with no layer check. Fixed with
`scanline_layer: [usize; 240]` written at every BG/OBJ write site in the mode
1-5 renderers and checked against BLDCNT's 1st-target bits, as `blend()` does.
Test `brighten_in_a_bitmap_mode_only_affects_the_selected_1st_target_layer`.

**Application**: a "scanline-wide approximation" comment in this PPU flags
skipped per-pixel layer gating.

### 2026-09-07 - A scanline drawn at the end of its line eats the HBlank handler's writes

From a Yggdra save state, two panels flickered between exactly two pictures
(11895 pixels) while `io`, `pal`, `vram` and `oam` hashed **identical** across
frames. The game's HBlank handler writes `BG1VOFS` once per line. `Ppu::tick`
drew the line at cycle 1232, but the HBlank IRQ fires at 960, so the write
landed on line N or N+1 depending on CPU/PPU phase. Now the line is drawn when
HBlank starts, with a `line_rendered` catch-up for a step that jumps over 960.
Test `a_register_written_during_hblank_takes_effect_on_the_next_line`.

**Application**: an effect one line off, or a two-frame flicker with the game's
memory unchanged, is render timing. Hash `io`/`pal`/`vram`/`oam` first;
identical hashes mean the bug is ours.

### 2026-09-07 - EEPROM's ready flag is polled by the CPU, not by DMA

Yggdra answered "Save failed!": six write commands, no reads. After a write
the save library spins on a plain `ldrh` of the EEPROM window for bit 0. Only
the DMA path consulted the chip; CPU reads fell through to the ROM mirror
(`0xFF80` on this 32 MB cart, whose window is `0x0DFFFF00-0x0DFFFFFF`). `read8`
now returns 1 across the window. Test
`a_cpu_read_of_the_eeprom_window_reports_the_chip_ready`.

**Application**: a save chip never read back is a handshake the game gave up
on. Check the poll path before the data path.

### 2026-09-07 - Games fake transparency by alternating frames; the LCD hid it

Yggdra's "SAVE DATA" title blinks: here `vram` and `oam` **do** change every
frame - the game redraws on alternating lines and the slow LCD smeared it.
`Ppu::set_interframe_blend` averages each frame with the previous one;
`prev_frame` keeps the *unblended* frame (or the picture fades), and the result
goes to a separate `blend_out` because `run_frame` can stop part way down the
next frame.

**Application**: before hunting a flicker, hash `vram`/`oam` across two
frames. Changing means the game is doing it on purpose.

### 2026-09-09 - Fast-forward speed is the game's business, not the emulator's

With the audio-clocked fast forward on a Mi 10T Pro, Yggdra hits 2.00x, 3.00x
and 4.00x exactly at 60 updates a second; Mario Tennis is CPU-bound before 2x
and gives ~1.5x at every ratio. Past the CPU limit a higher ratio is slower
and choppier (Yggdra peaks ~5.5x at ratio 6). Decision and numbers in
[`docs/research/fast-forward.md`](../docs/research/fast-forward.md).

**Application**: never quote one game's speed as the emulator's speed. An
earlier note took Mario Tennis's figure for a ceiling and was wrong by 3x on
the game the user plays. Measure two games and let the player pick the ratio.

### 2026-09-10 - GBATEK is wrong about sprite-over-sprite ordering

Mario Tennis's 3-2-1-GO digits (OAM 9-10, priority 0) were drawn behind panels
(OAM 1-2, priority 2), because the code followed GBATEK's Caution paragraph
("OBJ0 is always having priority above OBJ1-127"). VisualBoyAdvance bug #130's
test cannot isolate the case. mGBA (`software-obj.c`) and NanoBoyAdvance
(`sprite.cc`) both compare priority only and walk OAM upwards. The rule:
**priority decides, the OAM index only breaks an exact tie**. TONC's regobj
page was right. Tests
`a_sprite_with_a_better_priority_beats_a_lower_oam_index`,
`between_sprites_of_equal_priority_the_lower_oam_index_wins`.

**Application**: a confident sentence in GBATEK is not the end of the
argument. Check what mGBA and NanoBoyAdvance *do*, and when they disagree say
so in the comment. Only a hardware test ROM closes it completely.

### 2026-09-29 - Achievements: what the integration needs, and the two traps

PRs #30-#32. `docs/achievements.md` has the full reasoning.

1. **`rcheevos` belongs on the Android side, never in `core/`.** The core
   gained only `gbee_peek_memory`.
2. **The peek must not reuse `read8`**, whose EEPROM-window "chip ready"
   answer is right for a game and wrong for an observer. `peek` reads 0 there
   and is `&self`.
3. **rcheevos addresses a flat block, and a wrong map fails silently**:
   `0x000000-0x007FFF -> 0x03000000` (IWRAM), `0x008000-0x047FFF ->
   0x02000000` (EWRAM), `0x048000-0x057FFF -> 0x0E000000` (save).
4. **A ROM is identified by an MD5 of the whole file**: a renamed ROM matches,
   a trimmed or patched one does not.
5. **The runtime's state travels with save states**
   (`rc_client_serialize_progress`), or a load desyncs game and achievements.
6. **Hardcore is not a code decision**: approval plus six months public, and
   no save states, rewind or frame advance.

**Application**: two bugs were only findable by use: the client was gated on
the emulator library being loaded (nobody signs in with a game open), and
`stopEmulation` unloaded the session while the ROM reload guard meant it never
came back. Check anything "session lifecycle" against that guard.

### 2026-10-02 - Homebrew suites beyond gba-suite

`core/tests/homebrew_suites.rs` runs FuzzARM, ARMWrestler and the mGBA suite
from `temp/roms/` (fetch commands in its header). ARMWrestler 79/79; all five
FuzzARM ROMs are gates since the THUMB shift-by-register-0 N/Z fix (#58). The
mGBA suite's score history is the table in `ROADMAP.md`, Phase 4.5.

Two harness traps: FuzzARM writes its EWRAM dump marker *first* and the rest
between slow text draws, so wait for its key-down loop, not the marker. And
ARMWrestler draws verdicts as bitmap text; the harness reads the failure mask
at the entry of its `DrawResult` routine, found by byte pattern.

### 2026-10-02 - Timer IRQ 90/90 and I/O read 130/130: what the model assumes

- **The CPU's view of I/O and the hardware's latched view are two reads.**
  `MemoryBus::read8/16/32` apply `io::read_rule` (readable-bit masks, open bus
  for write-only registers). The PPU and APU need the write-only bits, so
  **any internal consumer of an I/O register must use `io_read8/io_read16`**;
  `bus.read16(0x0400_00xx)` from core code returns open bus for write-only
  registers.
- Open bus is GBATEK "Reading from Unused Memory": `Cpu::step` publishes PC
  and state into `bus.exec_pc/exec_thumb`, and `open_bus()` re-reads the
  prefetch.
- Timer/IRQ timing fitted to the suite's hardware values, all needed together:
  timer register writes apply *after* that instruction's timer tick; a newly
  started timer skips one more cycle (`Timer::delay_start`); IRQs are taken
  `IRQ_DELAY = 4` cycles after the line rises. The BIOS IRQ stub is the real
  sequence (`b 0x128` ... `ldr pc, [r0, #-4]`, so the user handler must be
  ARM); exception entry costs 3; ARM writes to PC pay the 2-cycle refill.
- **Save states carry no BIOS memory, so the HLE BIOS layout is a save-state
  format.** Moving the stub to 0x128 broke every quick save taken inside an
  IRQ handler under the old stub (LR = 0x28): Yggdra at 37 fps, 5400
  underruns. The old epilogue stays at 0x28/0x2C (`init_bios`); test
  `an_irq_handler_from_an_old_save_state_still_returns_through_0x28`. Any
  future BIOS-stub change must keep every address an old state can hold.

### 2026-10-02 - Per-step cost after the timer fix (#62)

- **The #60 slowdown was the per-step timer work**, not the I/O masks:
  publishing `TMxCNT_L` ahead every step ~4%, the count-by-count
  `Timer::tick` ~9%. Fixed by a no-overflow fast-forward in `Timer::tick`, a
  RAM/ROM fast path for `read16`/`read32` (`MemoryBus::plain`), and skipping
  empty write queues. Mario Tennis 9.40 -> 7.64 ms a frame on the host.
  Equivalence tests `ticking_a_slice_at_once_matches_ticking_it_cycle_by_cycle`,
  `wide_reads_match_the_byte_path`.
- **`Timer::tick`'s first-IRQ offset is the lowest-numbered timer's**, not the
  earliest, when two timers overflow in one slice. Left as is; the
  equivalence test documents it.
- Still per step by design: the four-timer publish into `io_regs` (the bus
  cannot see the timer), `apu.tick`, `ppu.tick`. Moving the timer into the bus
  is not worth ~3%.

### 2026-10-03 - Timer count-up 936/936: three causes, one fitted

- **HALT woke on PPU events only**, so a timer IRQ while halted woke up to
  ~1000 cycles late. `Gba::step` also stops at `Timer::cycles_to_irq()`. Test
  `a_timer_irq_ends_halt_on_the_overflow_cycle`.
- **Prescaled timers count on global prescaler edges** (`Timer::align_start`).
  The suite's table shows it (15 extra cycles cost 6 passes at /64 but gain 2
  at /256 and /1024); GBATEK is silent. Test
  `a_prescaled_timer_counts_on_the_global_prescaler_edges`.
- **HLE `IntrWait` returned ~45 cycles early after a wake**
  (`bios::INTR_WAIT_RETURN_CYCLES`, charged through `Cpu::entry_cycles`).
  Fitted but sharp (44: 16 fails, 45: 0, 46: 18). Only the after-a-wake return
  is charged.
- `timer_count_up_matches_the_mgba_suite` replays the suite's own code copied
  out of IWRAM, because the dispatcher's length is part of the result.

### 2026-10-04 - DMA 1244/1244: the DMA reads its own latch, and ROM always increments

- **A DMA read below EWRAM returns that channel's last transferred value**
  (`DmaChannel::latch`, per channel, halfwords latched on both halves). That
  covers the BIOS, unmapped `0x00004000-0x01FFFFFF`, and DMA0 reading ROM
  (its 27-bit mask folds 0x08 to 0x00). A halfword from a word latch takes the
  lane of `dest & 2`.
- **A GamePak ROM source always increments**, fixed and decrement included.
  GBATEK is silent; mGBA and the suite agree.
- **DMA drops the address bits below its unit width at the enable edge**,
  visible only on the 8-bit SRAM bus.

### 2026-10-04 - BIOS math 615/615, multiply long 72/72, Memory 1552/1552

- **The HLE ArcTan had three wrong coefficients** (0xFB60, 0x16C9, 0x3B10
  where the BIOS has 0xFB6, 0x16AA, 0x3651). Hardware returns 0x2000 for
  tan = 1.0. ArcTan uses all of r0 and returns a sign-extended halfword; the
  BIOS leaves r1 = `a`, r3 = `b`; ArcTan2 leaves r3 = 0x170; Div leaves r3 =
  |quotient|, and x / 0 returns sign(x), r1 = x, r3 = 1 (the real BIOS hangs
  for |x| > 1). `tests/bios.rs` embeds the suite's 123-row hardware table.
- **Multiply C is the Booth array's carry**, which mGBA does not model. The
  model is zaydlang's (`multiplication-algorithm`, zlib), translated in
  `cpu/arm.rs` `booth_carry`. **Never port from NanoBoyAdvance** (GPL-3.0).
- **BIOS read protection is a latch in the bus** (`MemoryBus::bios_latch`):
  refreshed from the prefetch while PC < 0x4000; the HLE SWI path sets
  0xE3A02004 (0xE129F000 for SoftReset); the IRQ stub's return prefetch word
  0xE55EC002 sits at 0x144. This made gba-suite's `bios.gba` pass.
- CpuSet/CpuFastSet copy nothing from a source below 0x02000000; 16-bit
  CpuSet from an odd source stores the `ldrh`-rotated halfword. ROM past its
  end reads `(address >> 1) & 0xFFFF` per halfword. An unaligned `ldr` from
  SRAM reads the byte replicated.

### 2026-10-07 - SIO 90/90 and 4/4, Misc. edge cases 6/12

- **The misc-edge suite's SRAM log has got/expected swapped**: "Got X vs Y"
  there means X = hardware.
- **SIO registers**: mode is RCNT bits 14-15 (GP/JOY), else SIOCNT bits
  12-13. SIODATA32 writable only in 32-bit normal mode; SIOCNT stores
  `& 0x7F8F` and reads idle pins (normal 0x04, multi 0x0C, UART empty 0x20);
  RCNT stores `& 0xC1FF`; JOYCNT bits 0-2 write-1-to-clear. `tests/sio.rs`
  embeds the 90-row table.
- **Only a normal-mode internal-clock transfer completes without a cable**:
  bits x 64 cycles (256 KHz) or x 8 (2 MHz), then data all ones, IRQ 7.
- **HLE `Halt` charges `HALT_RETURN_CYCLES = 30` after the wake**, fitted and
  sharp (29 and 31 fail).
- **DMA repeat reloads the count from DMAxCNT_L.** **DISPSTAT's H-Blank flag
  rises at line cycle 1006**; the IRQ, HBlank DMA and line draw stay at 960.
- Left: "DMA Prefetch" (2 cells) needs a DMA inside a load and a cycle-exact
  ROM loop; "H-blank bit" Flip 1-3 wants a wake ~35 cycles later, and a
  1235-vs-1232 delta between identical halts is unexplained.

### 2026-10-08 - Wait states, prefetch, DMA stalls: Timing 236 -> 1880/2020 (#69, #71)

Design and decisions in `docs/research/wait-states-design.md`.

- **Cycle accounting**: decoders return data + internal cycles; `Cpu::step`
  adds the opcode fetch and one central refill (N+S at the target) whenever
  PC was written. Costs come from a 256-entry table (`memory/timing.rs`)
  rebuilt by `set_waitcnt` on store, reset and save-state load. Measured
  traps: a running total in a struct field cost ~7% (store-forwarding chain),
  a boxed table ~3%, and a second `match op` in THUMB `format4_alu` ~6% (it
  broke the jump table). The refactor was proven neutral by hashing every
  step's (PC, cycles) over 1500 frames of eight ROMs.
- **Fixed counts**: THUMB hi-register ops cost 3 whatever they did; register-
  offset stores and `push` one too many; `pop {pc}` had no refill; multiplies
  ignored `m`; register shifts had no I.
- **Rules from the suite's table, not GBATEK** (`tests/timing.rs`): the opcode
  fetch after a data access is N (after a refill, S); a load samples I/O
  after its opcode fetch, so `TMxCNT_L` is published ahead by the next
  fetch's cost. Prefetch (`cpu/prefetch.rs`): fills one halfword per S16 on
  every cycle the CPU leaves the GamePak bus; a buffered fetch is 1 cycle; a
  GamePak data access empties it and pays +1 if the halfword in flight had 1
  cycle left; LDM/POP's I cycle comes after the transfers. DMA:
  2N + 2(n-1)S + 2I (GBATEK's 4I does not fit).
- **Stall placement**: an *immediate* DMA's stall lands after the next
  instruction when that one's fetch is 1 cycle (start latency 2). HBlank,
  VBlank and sound DMAs are resolved at the end of the step that started them
  (`Gba::step`, the loop after `post_tick`). `Gba::take_dma_stall` is the one
  place a stall is consumed; a GamePak DMA also sets `cpu.fetch_n`. The halted
  path clears stalls **after** `post_tick`, or the waking step's FIFO refill
  is charged to the first instruction after the wake.
- `Timer::counter_after` wraps modulo the period (`0x10000 - reload`).
- Games now get hardware CPU speed: Mario Tennis 142.5k -> 84k steps a frame,
  Yggdra +10% host cost. Celeste first draws on frame 12.
- Test traps: the FIFO requests a refill whenever it holds <= 16 bytes, so
  pre-fill a word to isolate one refill; detect an HBlank DMA's step with
  `gba.ppu.hblank`, not the DISPSTAT flag (1006 vs 960). The suite's SRAM log
  stops at 0x8000 bytes (`savprintf`); `temp/tprobe` rewinds its `location`
  to read the whole timing log.
- `mgba_suite_does_not_regress` gates every sub-suite on `MGBA_FLOOR` (raise a
  floor when a fix gains cells). ~3 s release, ~47 s debug.

### 2026-10-08 - Save state v10 carries the ROM CRC-32 (Phase 5)

- v10 appends the CRC-32 of the ROM (`cart::crc32`, same as
  `java.util.zip.CRC32`), cached in `Cartridge::rom_crc`. `restore_unchecked`
  checks it **last** (`SaveStateError::DifferentRom`); the rollback undoes the
  parse.
- Every tail-relative offset moves when the tail grows. Probes that hard-code
  tails edit the wrong bytes and report false "ok"s; `core/tests/state_restore.rs`
  has the current offsets as constants.
- Restore rejects what `tick`/`step` can never produce (`sample_accum >=
  GBA_CLOCK`, `sweep_shift > 7`, timer values `> 0xFFFF`, `cycles > 2^62`, an
  EEPROM `count` past its phase). Counters with no tight bound saturate.
- HLE BgAffineSet/ObjAffineSet cap r2 at `AFFINE_SET_MAX` (0x10000), and every
  guest-address add in `bios.rs` wraps.
- JNI `nativeStateWrite`/`nativeSaveWrite` call the C ABI functions, so
  `guarded` lives in one place; `tests/jni_exports.rs` checks it by reading
  `ffi.rs`.

### 2026-10-11 - Building the Android app in a git worktree

- A new worktree has an empty `android/app/src/main/cpp/rcheevos/` submodule
  and no `jniLibs`. Run `git submodule update --init
  android/app/src/main/cpp/rcheevos` and copy `android/app/src/main/jniLibs`
  from the main checkout (gitignored).
- If CMake already configured against the empty submodule, delete
  `android/app/.cxx` and `android/app/build/intermediates/cxx`, or the link
  fails with undefined `rc_client_*`: the source glob was cached empty.
- Never copy the main checkout's rcheevos folder in: its `.git` pointer file
  breaks every git command in the worktree.

### 2026-10-11 - ROM folders are tree Uris or legacy paths (fix/saf-folders)

- `rom_folders` prefs hold document-tree Uri strings and absolute paths; both
  scan. A ROM id is `buildDocumentUriUsingTree(tree, docId)` (same builder in
  every scan, so keys are stable). Re-picking exactly `primary:<rel>` replaces
  the path folder and re-keys LastPlayed and Favorites by `rekeyedDocId`
  (`data/RomFolders.kt`, JVM-tested).
- What sits beside a document ROM (`.sav`, cover) is recorded by the scan into
  `RomSiblings`. A missing key means "not scanned", NOT "no .sav":
  `RomFiles.legacySaveFor` returns null and callers must skip the import,
  because `importLegacySave` writes its marker when the legacy side is absent.
- `importLegacySave` takes the legacy side as size/mtime/bytes; a provider that
  reports no SIZE is measured by reading, never treated as 0.
- Unavailable tree folders are `RomFolderManager.unavailable`; a folder is
  never auto-removed.

### 2026-10-12 - Save import core

- `Cartridge::load_save` 0xFF-fills the whole chip before copying, so a short
  file reads erased past its end. Save-state restore goes through it too.
- `cart::save_fit(rom, len)` is the import size rule (exact chip size or +16
  mGBA RTC footer; EEPROM accepts 512 and 8192). C `gbee_save_fit`, JNI static
  `GbaEngine.nativeSaveFit`.
- `core/tests/jni_exports.rs` only finds Kotlin externals on lines that START
  with `external fun` or `private external fun`. Put `@JvmStatic` on the line
  above, or the export check silently skips the declaration.

### 2026-10-14 - Save state v11: live state between steps (#94)

- A save state is taken **between two `Gba::step` calls**, so anything a step
  leaves set for the next one is machine state, however "transient" its
  comment says. v11 appends 43 bytes after the CRC: PPU `cycle_counter`,
  `line_rendered`, the four affine `bg*_internal`, `io.intr_wait_active`,
  `io.halt_swi`, `bus.bios_latch`, `cpu.entry_cycles`, `Gba::dma_stall`,
  `Gba::irq_since` (`u64::MAX` = None).
- Derived, not saved: the PPU's DISPSTAT copies, re-read by
  `sync_dispstat_cache` on restore. v6-v10 states get `cycle_counter` from
  `hblank`, affine reloaded from BGxX/Y, the rest power-on values.
- Deliberately unsaved (ceiling: a few cycles once): `cpu.fetch_n`, the
  prefetch buffer, `timer.start_delay`.
- `core/tests/state_resume.rs` saves at every step boundary of a timer-IRQ
  loop and compares traces. Use it when adding cross-step state.
- A layout change moves tail offsets in `state_restore.rs` (`LIVE_LEN`),
  `saves.rs` (absurd-length test), `savestate.rs` unit tests, and
  `StateTransfer.kt` `STATE_VERSIONS`.

### 2026-10-17 - Phase 5 boundary re-audit

- The v10 ROM CRC check only runs for `version >= 10` (`savestate.rs`), so a
  state whose version field says 6..9 skips it; `StateTransfer.kt` imports
  any 6..11 state (P5-RE-1, `temp/phase5/rust-engineer.md`).
- rcheevos lists hold raw pointers into the loaded game and
  `rc_client_unload_game` frees it after dropping its mutex: build *and read*
  a list under the lock that serialises unload (`RaEngine.coreLock`) (P5-RE-2).
- `nativeServerResponse` has no size cap in C and the body is not
  NUL-terminated; the 16 MiB cap is Kotlin-only (`MAX_RESPONSE_BYTES`).

### 2026-10-27 - Pokemon Emerald: affine sprites and PSG sound (#112)

- **ObjAffineSet/BgAffineSet were stubs** writing pa=0x100, pb=0 and never
  pc/pd. Emerald calls SWI 0x0F every frame (r3 = 2, packed matrix), so its
  affine sprites drew with stack garbage as pc/pd. The docs had called them
  "identity matrix" stubs, which understated it. Now real: 1.14 sine table,
  `asr #14`, pb negated after the shift (`bios.rs` `affine_matrix`). The sine
  table is computed, not read from a BIOS dump, and may be 1 LSB off on some
  angles. Tests `obj_affine_set_*`, `bg_affine_set_*`.
- **The APU frame sequencer ran at 1702 Hz** (every 9856 cycles) for envelope,
  sweep and length. m4a uses the hardware envelope for square instruments, so
  notes died ~26x too fast. Now 512 Hz, length/sweep/envelope on steps
  even/2,6/7; `envelope_tick_counter` is the position in the 8-step cycle.
  The Phase 1 APU ticks had been "verified" by tests that checked only trigger
  and pitch.
- The mixer ignored SOUNDCNT_L (routing, master volume) and SOUNDCNT_H bits
  0-1, the wave channel was 2x the others, mono summed instead of averaging,
  and a square's duty left a DC offset. `tests/apu.rs` covers each.
  `Apu::resync_mixer` re-derives the mixer from latched SOUNDCNT_L/H after a
  restore, so old states sound right.
- **Comparing audio against mGBA works headless**: `SDL_VIDEODRIVER=dummy
  SDL_AUDIODRIVER=disk SDL_DISKAUDIOFILE=<abs path>.raw timeout 45
  temp/mgba/run.sh <rom>` writes 44.1 kHz stereo s16 of the boot and title.
  Compare 20 ms RMS envelopes after aligning by cross-correlation
  (`temp/emerald/scripts/cmp.py`). Emerald title: correlation 0.665 -> 0.980.

### 2026-10-27 - Baldur's Gate DA: bitmap modes ignored the BG2 enable (#113)

`ppu/mod.rs` drew modes 3-5 without checking DISPCNT bit 10. BG:DA's game-over
and cutscene-text screens are mode 4 with BG2 off and the text in sprites, so
VRAM tile data was drawn full-screen as noise. Test
`a_bitmap_mode_with_bg2_off_shows_the_backdrop`. mGBA gates the same way
(`video-software.c`, `TEST_LAYER_ENABLED(2)`). Probe bins in `temp/emerald/probe`
take `ROM=<path>`; `lines` logs PPU registers per scanline.

### 2026-10-27 - PPU, APU and HLE BIOS bugs are found by players, not suites

Every CPU, bus, timer and DMA bug since 2026-10-02 was caught by a test suite;
every PPU, APU and HLE BIOS bug was found by someone playing a game. The free
suites wired here have no PPU, APU or affine-set coverage. The audit by bug
class, with what to test next, is
[`docs/research/gba-bug-classes.md`](../docs/research/gba-bug-classes.md); the
owned, prioritised list is `ROADMAP.md`, Known accuracy gaps. Durable facts
from the 2026-10-27 core audit, each confirmed in code:

- **FIFO samples are lost while the CPU is halted.** The halted branch of
  `Gba::step` (`core/src/lib.rs:141-153`) runs `apu.tick(step)` before
  `post_tick` drains timer overflows into `Apu::on_timer_overflow`
  (`core/src/apu/mod.rs:577`), and halted steps reach 960 cycles because the
  sound timer has no IRQ to bound them. Several overflows collapse into one
  heard pop; a probe at 24 kHz heard 197 of 401 pops a frame.
- **A PSG frequency-byte write retriggers the note.** The trigger bit stays
  latched in `io_regs`, and `Gba::apu_sound_write`
  (`core/src/lib.rs:439-458`) re-reads the whole halfword on every byte write,
  so m4a's lone `strb` to the frequency low byte restarts the envelope.
- **ROM-dependent tests are not pins.** `gba_suite`, `homebrew_suites`, the
  ROM cases in `ppu.rs`, `integration.rs`, `input_slices.rs` and `tonerom.rs`
  print "skipping" and **pass** when their ROM is missing from `temp/roms/` -
  which it always is on a fresh clone and in CI. Behaviour that only those
  tests cover is unpinned there.
- An unhandled SWI is a silent no-op: `Cpu::swi` (`cpu/mod.rs`) discards
  `handle_swi`'s result and logs nothing.

**Application**: before calling a PPU, APU or BIOS feature done, find a test
that pins it without a ROM, or compare against mGBA (screenshot or audio
envelope). A known-gaps list without an owner, priority and affected games is
where debt goes to sit.
