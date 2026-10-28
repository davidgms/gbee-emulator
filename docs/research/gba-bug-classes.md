# GBA accuracy bug classes

Written 2026-10-27 from five research lanes (A: graphics bugs other emulators
fixed, B: sound bugs, C: community reports, D1: GBee's own fix history, D2: a
stub and approximation audit of `core/` at 81ff215). The lane notes live in
`temp/bug-research/` and are not committed; every claim this doc depends on is
quoted here with its source. Android, pacing, MIUI and audio-clock topics are in
[`emulators-research.md`](emulators-research.md) and are not repeated.

## 1. Why this exists

On 2026-10-27 testers hit three bugs in one day: Emerald affine sprites drawn as
lines (ObjAffineSet/BgAffineSet stubs, PR #112), Emerald music with notes dying
~26x fast (APU frame sequencer at 1702 Hz, SOUNDCNT_L routing and volume
ignored, #112), and the Baldur's Gate DA game-over screen showing tile garbage
(modes 3-5 ignored the DISPCNT BG2 enable, #113).

D1's verdict over 49 fixes: every CPU, bus, DMA and timer bug since 2026-10-02
was caught by a wired suite (jsmolka gba-suite, mGBA suite, FuzzARM). Every PPU,
APU and HLE BIOS bug was found by a person playing a game, because no wired
suite covers those areas. The affine stubs had been listed as debt in
`ROADMAP.md` since 2026-08-28; the Known accuracy gaps list has no owner,
priority or games-affected column, so nothing scheduled them.

## 2. Class matrix

Ext = rows in lanes A+B (external emulator fixes). C = community reports in
lane C. D1 = GBee fix rows. Status from D2: impl+test / approx / stub / absent /
unverified (no pinning test). ROM-gated tests skip on a fresh clone and do not
count as pins.

| Class | Ext | C | D1 | GBee status now | Covering test | Risk |
|---|---|---|---|---|---|---|
| PPU.objwin | 5 | 2 | 1 | impl, unverified (shape gating untested) | `dispcnt_bit_15_is_the_obj_window_not_a_display_off` (no blackout only) | med |
| PPU.window | 3 | 2 | 2 | approx: X1>X2 clamped; bitmap modes absent | `a_window_hides_the_layers_winout_leaves_out`, `win0_takes_precedence_over_win1` | med |
| PPU.mosaic | 8 | 1 | 2 | text BG impl; OBJ approx; affine/bitmap BG absent | `mosaic_only_applies_to_...`, `obj_mosaic_only_applies_to_...` | med |
| PPU.blend | 5 | 1 | 4 | mode 0 impl (1 LSB off); modes 1-5 approx, semi-transparent OBJ lost | `alpha_blend_mixes_...`, `a_semi_transparent_sprite_blends_...` | med |
| PPU.bitmap | 3 | 1 | 1 | approx: BG2 gate pinned; mode 4 index 0 opaque; mode 5 outside area black | `a_bitmap_mode_with_bg2_off_shows_the_backdrop` | med |
| PPU.bitmap-affine | 0 | 0 | 0 | absent (BG2 PA-PD, BG2X/Y ignored in 3-5) | none | high |
| PPU.affine (BG) | 3 | 1 | 1 | impl; wrap mode unverified; reload only on changed value | `an_affine_background_reads_its_tilemap` et al. | low |
| PPU.affine-obj | 1 | 1 | 0 | approx: texel truncates toward zero, not floor | none | high |
| PPU.priority | 2 | 1 | 3 | mode 0/1/2 BG impl; OBJ vs BG in modes 1-5 absent | `a_low_priority_sprite_goes_behind_the_background` (mode 0) | high |
| PPU.midframe | 3 | 4 | 3 | approx (per-line latch) | `a_register_written_during_hblank_takes_effect_on_the_next_line` | low |
| OBJ.limits | 6 | 1 | 0 | absent (all 128 OBJs always drawn) | none | low |
| OBJ.vram-bounds | 2 | 1 | 1 | impl, unverified | `sprites_read_their_tiles_from_obj_vram` (base only) | low |
| BG.text-wrap | 2 | 0 | 0 | impl, unverified | none | low |
| IRQ.timing | 7 | 1 | 5 | impl (fitted to one mGBA suite) | `timer_reads_and_irq_latency_match_the_mgba_timer_irq_suite` | low |
| DMA.timing | 12 | 1 | 7 | impl (atomic transfers); video capture absent | `dma_stalls_the_cpu_as_on_hardware` et al. | low |
| TIMER | 0 | 0 | 6 | impl | `timer_count_up_matches_the_mgba_suite` et al. | low |
| CPU.* | 0 | 0 | 12 | impl | `cpu.rs`, `gba_suite.rs` gate | low |
| BIOS.swi-* | 10 | 2 | 9 | affine sets, BitUnPack, HuffUnComp, GetBiosChecksum, MidiKey2Freq, RegisterRamReset impl (2026-10-28); 0x03, 0x1A-0x1E, 0x20-0x2A absent, logged once per number | `bios.rs` (math table, decompressors, affine, `bit_unpack_*`, `huff_uncomp_*`, `register_ram_reset_*`, `an_unhandled_swi_is_reported_once`) | med |
| APU.fifo-dma | 10 | 3 | 2 | impl; refill polled at <=16 bytes; master-off does not reset FIFOs | `dma_sound_plays_at_the_rate_its_timer_overflows` | med |
| APU.timer-tick | 4 | 0 | 1 | impl (2026-10-28): each FIFO byte plays from its overflow's cycle, halted or in a DMA stall | `dma_sound_pops_every_overflow_while_the_cpu_is_halted`, `dma_sound_pops_every_overflow_during_a_long_dma` | low |
| APU.mix-bias | 7 | 0 | 2 | approx: SOUNDBIAS never read | `the_psg_master_volume_scales_by_an_eighth_per_step`, `soundcnt_h_scales_the_psg` | med |
| APU.psg-trigger | 1 | 0 | 0 | impl (2026-10-28): trigger is a write-only strobe | `a_frequency_low_byte_write_does_not_retrigger_the_channel`, `the_trigger_bit_is_write_only_on_every_psg_channel` | low |
| APU.psg-env | 6 | 0 | 1 | approx: any CNT_H write reloads envelope; no DAC-off | `the_envelope_steps_at_64_hz` | med |
| APU.psg-sweep | 4 | 0 | 1 | buggy: shift 0 writes freq back [PROBED] | none | med |
| APU.status (SOUNDCNT_X bits 0-3) | 1 | 0 | 0 | absent (`io.rs:194` comment only) | none | med |
| APU.wave | 5 | 0 | 1 | impl; CNT_L bit 7 starts playback without trigger | `wave_ram_writes_reach_the_bank_that_is_not_playing` et al. | low |
| APU.noise | 3 | 0 | 0 | approx, unverified (period and 7-bit width unpinned) | amplitude only | low |
| APU.resample / stereo | 5 | 3 | 1 | approx: point-sampled, mono `(L+R)/2` | `the_mono_output_averages_the_two_sides` (pins the gap) | med |
| APU.m4a | 4 | 1 | 0 | runs natively; inherits timer-tick and trigger bugs | none at ROM level | high |
| SAVE.* | 0 | 8 | 4 | SRAM/EEPROM impl; flash program overwrites (no AND); detection by string scan, no override DB | `saves.rs`, gba_suite save (ROM-gated) | med |
| RTC / GPIO | 0 | 5 | 0 | RTC impl (2026-10-28, `cart/gpio.rs`); time writes dropped, no RTC IRQ; solar/gyro/rumble absent | `tests/rtc.rs` | med |
| QUIRK | 8 | 4 | 3 | mixed | per item | low |

Counts are lower bounds from tracker samples skewed toward mGBA; use them to
rank, not as a census.

## 3. Open risks, ranked

Ranked by games broken x severity x how invisible the failure is. GBATEK is
https://problemkaputt.de/gbatek.htm, section named. Line numbers are at 81ff215.

### 1. FIFO samples lost while the CPU is halted (APU.timer-tick)

**Fixed 2026-10-28.** `Timer::increment` records where in the slice timers 0
and 1 overflowed (`sound_overflow_at`), and `Gba::tick_apu` renders the APU up
to each overflow before popping the FIFO, on all three tick paths (running,
halted, late DMA stall). The same collapse hit the running CPU during a long
DMA stall, which the original write-up missed: a 320-word DMA3 per step left 34
of ~401 sign changes. Correction to "Games" below: **Emerald never halts** -
pokeemerald's `WaitForVBlank` busy-waits on `gMain.intrCheck` (0 halted steps
measured over 600 frames) - so Emerald was hit through DMA stalls, not HALT.
Tests: `apu.rs` `dma_sound_pops_every_overflow_while_the_cpu_is_halted` (196 ->
401) and `dma_sound_pops_every_overflow_during_a_long_dma` (34 -> 401).

- Symptom: grit and aliasing on DMA-sound music. Probe at 24 kHz: 401 FIFO pops
  per frame audible with the CPU running, 197 while halted.
- Where: `core/src/lib.rs:131` (halted step is `cycles_to_irq().min(ppu event)`;
  the sound timer raises no IRQ, so steps reach 960 cycles), `lib.rs:144`
  (`apu.tick(step)` renders the step's samples first), `lib.rs:406`
  (`on_timer_overflow` runs afterwards, once per drained overflow),
  `core/src/apu/mod.rs:577`. Every overflow but the last in a step is never heard.
- Source: GBATEK "DMA Sound"; B#15 pret `m4a.c` (timer 0 reload
  `-(280896 / pcmSamplesPerVBlank)`); B#1 mGBA #356 (FIFO phase vs timer is audible).
- Games: nearly every commercial game (all park in VBlankIntrWait); Emerald and
  every m4a title first.
- Failing test (to write): `core/tests/apu.rs`: `dma_sound_pops_every_overflow_while_the_cpu_is_halted`
  (timer at 700 cycles, DMA1 to FIFO A, CPU halted; sign flips == overflows +-1).

### 2. Byte write to a PSG frequency register re-triggers the note (APU.psg-trigger)

**Fixed 2026-10-28.** `Gba::apu_sound_write` masks bit 15 of SOUND1CNT_X,
SOUND2CNT_H, SOUND3CNT_X and SOUND4CNT_H unless the byte written is the high
byte with bit 7 set, and clears the bit from `io_regs` after use. mGBA 0.10.4
has the same fix. Tests: `apu.rs` `a_frequency_low_byte_write_does_not_retrigger_the_channel`,
`the_trigger_bit_is_write_only_on_every_psg_channel`.

- Symptom: m4a PSG vibrato and pitch bends restart the note and its envelope.
  Probe: envelope decayed to 0, one `write8(0x04000064)`, volume back at 14-15.
- Where: `core/src/memory/mod.rs:664` keeps the write-only trigger bit in
  `io_regs`; `core/src/lib.rs:456` re-reads the whole latched halfword on every
  sound byte write and passes it to `apu.write_register`.
- Source: GBATEK "Sound Channel 1-4" (bit 15 write-only); B#31 mGBA #3086,
  "Fix audio register 8-bit write behavior" (Emerald potion heal sound);
  B#29 NBA #389 (same sound, frequency writes during a note).
- Games: every m4a game (CgbSound stores NRx3 as its own byte), Emerald and
  FireRed heal sound named externally.
- Failing test (to write): `core/tests/apu.rs`: `a_frequency_low_byte_write_does_not_retrigger_the_channel`.

### 3. RTC and cartridge GPIO absent (RTC)

**Fixed 2026-10-28** (RTC only). `core/src/cart/gpio.rs`: detection by game
code (AXV, AXP, BPE, U3I, U32, U33, BR4, BKA) or `SIIRTC_V`; GPIO data,
direction and control; S-3511 reset, control, date/time and time, bits LSB
first with the command's fixed code in the low nibble (mGBA #1721). Power-on
control 0x40. Time is the host clock plus an offset the frontend sets with
`Gba::set_rtc_time` / `gbee_set_rtc_time` / JNI `nativeSetRtcTime`; date and
time writes are dropped, no RTC IRQ. The readable port is patched into the
bus's ROM copy so opcode fetches pay nothing. Save state v12 carries the block.
Tests: `core/tests/rtc.rs` (5), `savestate.rs` `a_version_11_state_still_loads_with_a_set_up_rtc`,
`a_hostile_rtc_state_is_rejected`. Boktai's solar sensor, gyro and rumble are
still absent.

- Symptom: Ruby/Sapphire/Emerald show "The internal battery has run dry"; berries,
  tides, Shoal Cave and daily events never happen. Boktai solar sensor, gyro and
  rumble carts also unsupported.
- Where: nothing exists; `core/src/cart/mod.rs:622` only drops mGBA's RTC save
  footer. The bus has no handler at 0x080000C4-0x080000C9.
- Source: GBATEK "GBA Cart Real-Time Clock (RTC)" and "GBA Cart I/O Port (GPIO)";
  C: gpSP #275 (battery-dry message), mGBA #1721 (RTC command bit order; the
  stuck-night symptom itself is the hack's code), Lemuroid #1153 (Emerald save
  handling on an RTC cart), SkyEmu #183 (Boktai).
- Games: Pokemon Ruby, Sapphire, Emerald (the project's headline game), Boktai 1-3.
- Failing test (to write): `core/tests/saves.rs`: `an_rtc_cart_answers_the_gpio_status_command`
  (enable GPIO at 0x080000C8, send 0x63 over the 3-wire protocol, assert a valid status byte).

### 4. Unhandled SWIs are silent no-ops (BIOS.swi-*)

**Fixed 2026-10-28.** `bios::report_unhandled` warns once per SWI number.
Added GetBiosChecksum (0x0D), BitUnPack (0x10, was a stub), HuffUnComp (0x13,
4- and 8-bit), MidiKey2Freq (0x1F, single precision as mGBA). Still absent:
0x03 Stop, 0x1A-0x1E, 0x20-0x2A. Tests: `bios.rs` `an_unhandled_swi_is_reported_once`,
`get_bios_checksum_returns_the_gba_value`, `bit_unpack_*` (2),
`huff_uncomp_*` (2), `midi_key2freq_returns_the_gbatek_formula`.

- Symptom: a call to an absent SWI returns with r0-r3 unchanged and no log line.
  Huffman assets become blank memory, MidiKey2Freq returns its input as a pitch,
  GetBiosChecksum fails anti-tamper checks, and nobody can tell from a log.
- Where: `core/src/bios.rs:33` (`_ => false`), `core/src/cpu/mod.rs:655`
  (`handle_swi`'s bool discarded). Absent: 0x03 Stop, 0x0D GetBiosChecksum,
  0x13 HuffUnComp, 0x1A-0x24, 0x25 MultiBoot, 0x26/0x27, 0x28-0x2A, plus
  0x10 BitUnPack as a stub (`bios.rs:458`, copies until a zero byte).
- Source: GBATEK "BIOS Functions" (per-SWI sections); C: mGBA #3248 (Radical Red,
  "This game uses a BIOS call that is not implemented"), mGBA #3364 (Quetzal);
  B#53 mGBA `bios.c` logs a STUB for each unimplemented call.
- Games: Pokemon ROM hacks (Radical Red, Quetzal), text-heavy games using
  BitUnPack for 1bpp fonts, any title with Huffman data.
- Failing tests (to write): `core/tests/bios.rs`: `an_unhandled_swi_is_reported_once`
  (needs a counter or test logger), then `huff_uncomp_decodes_a_4bit_tree` and
  `bit_unpack_expands_1bpp_to_4bpp_with_offset`.

### 5. RegisterRamReset wipes the stack (BIOS.swi-RegisterRamReset)

**Fixed 2026-10-28.** Keeps IWRAM 0x7E00-0x7FFF, always forces DISPCNT =
0x0080, and bits 5-7 reset the SIO, sound and other registers through the bus
(register lists as mGBA). Tests: `bios.rs` `register_ram_reset_keeps_the_last_0x200_bytes_of_iwram`,
`register_ram_reset_resets_the_io_register_groups`.

- Symptom: bit 1 clears all 32 KB of IWRAM, including the stack and the IRQ
  vector at 0x03007FFC; a caller that pushed LR before `swi 1` returns to 0.
  Bit 0 clears all of EWRAM. Bits 5-7 (SIO, sound, other I/O) are ignored, so
  sound state survives a reset that should clear it.
- Where: `core/src/bios.rs:66-80` (`iwram_data_mut().iter_mut()` at :74).
  SoftReset already does it right at `bios.rs:51` (`[0x7E00..]`).
- Source: GBATEK "BIOS Reset Functions" ("except last 200h bytes"); A#65 mGBA
  #1756 (renderer state not reset on RegisterRamReset, Mario & Luigi).
- Games: pokeemerald calls RegisterRamReset(RESET_ALL) at boot and survives only
  because its wrapper keeps LR in a register; any game calling it from C with LR
  on the stack crashes.
- Failing test (to write): `core/tests/bios.rs`: `register_ram_reset_keeps_the_last_0x200_bytes_of_iwram`.

### 6-18. Next tier

| # | Risk | Where | External evidence | Failing test to write (file: name) |
|---|---|---|---|---|
| 6 | OBJ vs BG priority ignored in modes 1-5 (sprites over foreground BGs, HUDs, bridges) | `ppu/mod.rs:636-645` | GBATEK "LCD OBJ - OAM Attributes" | `ppu.rs: mode1_a_low_priority_sprite_goes_behind_a_higher_priority_bg` |
| 7 | Affine OBJ texel `/256` truncates toward zero; seam and 1 px shift on every scaled sprite | `ppu/mod.rs:1165-1172` | GBATEK "OAM Rotation/Scaling"; Emerald battle intro, evolution | `ppu.rs: an_affine_sprite_at_2x_maps_each_texel_to_two_pixels` |
| 8 | BG2 affine regs ignored in modes 3-5 (mode 5 never scales full screen, FMVs) | `ppu/mod.rs:1313-1380` | A#22 mGBA #1668 Pinball Tycoon | `ppu.rs: mode3_honours_bg2x_scroll` |
| 9 | SOUNDCNT_X bits 0-3 (channel active) not modelled; engines that poll them cut notes | `io.rs:194` | B#25 SkyEmu #315 Minish Cap unlock jingle | `apu.rs: soundcnt_x_reports_a_playing_channel_until_its_length_expires` |
| 10 | Sweep shift 0 writes freq+freq back, kills the channel | `apu/mod.rs:874` | B#26 mGBA/NBA source agree: no write-back at shift 0 | `apu.rs: a_sweep_with_shift_zero_leaves_the_frequency_alone` |
| 11 | Envelope reloaded by any SOUNDxCNT_H write (duty or length byte) | `write_duty_envelope` (`apu/mod.rs:472`, reloads at :485, :493) | B#30 mGBA #866 (Metroid Fusion), CH 0.10.3 #3067 | `apu.rs: changing_the_duty_does_not_reset_the_envelope` |
| 12 | Windows do not mask BG2 in modes 3-5 | `ppu/mod.rs:1313-1380` | GBATEK "Window Feature" | `ppu.rs: win0_hides_the_bitmap_in_mode3` |
| 13 | Semi-transparent OBJ flag dropped in modes 1-5 | `ppu/mod.rs:636-645` | A#18 mGBA #2489; C mGBA #3804 | `ppu.rs: a_semi_transparent_sprite_blends_in_mode1` |
| 14 | FIFO: refill polled at <=16 bytes, not on request; master-off does not reset; overflow behaviour unmodelled | `lib.rs:383-401`, `dma.rs:340` | B#5 mGBA #1847 (7-word ring, reset on overflow), B#2 SkyEmu #61, B#3 NBA #101 (Beyblade) | `apu.rs: a_fifo_overflow_empties_the_fifo` |
| 15 | Save detection by string scan, no override DB; flash program overwrites instead of AND | `cart/mod.rs:680`, :406 | C class 3: libretro/mgba #136, Lemuroid #815, mGBA #2862 | `saves.rs: flash_program_can_only_clear_bits` |
| 16 | Mono output and point-sampled resampling (panning lost, high-note aliasing) | `apu/mod.rs:707-826`, :820 | B#44 (blip-buf, NBA sinc); but see section 4 on noise | `apu.rs: a_channel_routed_left_only_is_silent_on_the_right` (replace the mono-pinning test first) |
| 17 | **Fixed 2026-10-28** (see item 4): MidiKey2Freq (0x1F) and GetBiosChecksum (0x0D) absent | `bios.rs:9-33` | B#54 mGBA `_MidiKey2Freq` formula | `bios.rs: midi_key2freq_returns_the_gbatek_formula` |
| 18 | Affine BG reference reloads only when BGxX/Y changes; HBlank tables rewriting the same value keep accumulating | `ppu/mod.rs:406` | GBATEK "BG Rotation/Scaling" (internal ref regs); C mGBA #190 Madden | `ppu.rs: rewriting_bg2x_with_the_same_value_reloads_the_reference` |

Low, not ranked: OBJ cycle budget (A: Gunstar Super Heroes, Metroid ZM, Drill
Dozer), mosaic on affine and bitmap BGs, mode 5 outside-area black, DMA video
capture, HALTCNT STOP, SOUNDBIAS register and SWI 0x19, CpuSet register clobber
(A#63 mGBA #1531), HLE SWI timing (A#59 Advance Wars, A#62 Batman Vengeance).

## 4. Disputed or unknown behaviour

Rule: **no fix in this list without a hardware test ROM result or a GBATEK
passage.** Emulators copy each other; two agreeing is not evidence.

| Item | What sources say | Status in GBee |
|---|---|---|
| Sweep shift 0 overflow check | mGBA and NBA agree shift 0 writes nothing back, but differ on whether the overflow check still runs. GBATEK only says sweep time 0 disables. No test ROM named (B#26) | Fix the write-back (agreed); leave the overflow check until a hardware test |
| Zombie envelope mode | mGBA emulates it (CH 0.6.0, 0.10.3, code TODO "just AGB?"); NBA 1.8.0 removed it "does not appear to exist on GBA" (B#32) | Not implemented; keep it that way |
| Window X1>X2 / Y1>Y2 | GBATEK text reads as clamp to 240; D2 reports mGBA wraps; A#6 mGBA #1945 says start/end are stateful conditions (`window_midframe` test) | Clamped. Needs search-specialist check plus a test ROM |
| Noise band-limiting | mGBA #1265: super-sampling made FR/LG noise "very quiet" and was "incorrect for GBA audio"; the earlier version produced garbage in some cries | Any band-limited resampler must leave high-divisor noise aliased; compare against mGBA on FR/LG wild theme |
| DMA count latch | mGBA #3619: maintainer's latch explanation "was wrong after all", but the fix stands; NBA and gpSP match hardware (Sangokushi Eiketsuden) | Do not port mGBA's model; read NBA's |
| MP2K stereo FIFO mapping | NBA `apu.cc` TODO: assumes right = FIFO A, left = FIFO B, "haven't verified" (B#52) | Matters once stereo lands; pan comes from SOUNDCNT_H per FIFO |
| BIOS sine table | `bios_sin` computed with f64, may be 1 LSB off (f5f0f1c, `bios.rs:368`); no BIOS dump to compare | Approximate |
| Initial wave RAM | mGBA #2947 changed power-on contents; page does not say which values are right for GBA (B#36) | Unverified |
| FIFO overflow reset | mGBA #1847 states it as an observation; NBA 1.8.0 adopted it (B#5, B#6) | Two sources plus a test ROM (directaudiotest); acceptable basis |
| Hardware-true glitches | SkyEmu #53 NFS Porsche menu flicker occurs on hardware; mGBA #1695 Golden Sun Tret OOB seen on hardware | Do not "fix" |

## 5. Test-game watchlist and test ROMs

### Games per class (merged A, B, C)

| Class | Games |
|---|---|
| PPU.objwin | Zelda Minish Cap (lantern), Sonic the Hedgehog Genesis (zone titles) |
| PPU.window | Combat Choro Q, Pokemon Emerald (battle comb transition), Balatro GBA (homebrew) |
| PPU.mosaic | Castlevania Aria of Sorrow (intro), Super Mario Advance 2, Kingdom Hearts CoM |
| PPU.blend | Kirby Amazing Mirror (intro flash), Yoshi's Island |
| PPU.bitmap / bitmap-affine | Pinball Tycoon, Zidane Football 2002, Baldur's Gate DA (game over), any FMV title |
| PPU.affine | Rayman 3, Madden NFL 06/07 (coin toss), Zelda ALttP (512x512) |
| PPU.affine-obj | Pokemon Emerald (battle intro, Pokeball, evolution) |
| PPU.priority | Golden Sun 1/2, Minish Cap, Mario Tennis (countdown), Mario & Luigi |
| PPU.midframe | Yggdra Union, Final Fantasy I&II, Gunstar Super Heroes, Harry Potter PoA, FF VI Advance |
| OBJ.limits | Gunstar Super Heroes, Metroid Zero Mission (power bomb HUD), Famicom Mini series, Drill Dozer |
| IRQ.timing | Metal Max 2 Kai, James Pond Robocod, Hamtaro Ham-Ham Heartbreak, Wario Land 4 |
| DMA.timing | Lufia Ruins of Lore, Sangokushi Eiketsuden, Mother 3, Konjiki no Gash Bell 2 |
| BIOS.swi-* | Advance Wars 1/2, FF I&II (Div), Batman Vengeance (LZ77), Pokemon Radical Red, Quetzal |
| APU.fifo-dma | Beyblade V-Force, Pac-Man World 1/2, Super Robot Wars J, Splinter Cell Pandora Tomorrow, Yu-Gi-Oh Dungeondice Monsters |
| APU.psg-* | Pokemon Emerald/FireRed (heal sound), Metroid Fusion, Minish Cap (unlock jingle) |
| APU.noise | Pokemon FireRed/LeafGreen (wild theme), Metroid Zero Mission (Mother Ship) |
| APU.mix-bias | Fire Emblem Sacred Stones (SFX vs music), Konami Krazy Racers, BIOS boot jingle |
| APU.m4a | Pokemon Emerald (title, Littleroot), WarioWare Inc (track-start pop), Mother 3 |
| SAVE.* | Tactics Ogre (EEPROM512), Pokemon Mystery Dungeon Red (JP save type), Pokemon Unbound |
| RTC | Pokemon Ruby/Sapphire/Emerald, Boktai |

### Test ROMs to wire

None of these is ever committed (`.gitignore` blocks `*.gba`, `*.bin`,
`*.bios`). Each test points at `temp/roms/` and skips when the file is missing,
as `gba_suite.rs` and `homebrew_suites.rs` already do. A skip is not a pass:
report skipped suites in the test output.

| ROM | Covers | Named in |
|---|---|---|
| Tonc demos (`obj_aff`, `mos_demo`, affine BG demos) | 128 px OBJ wrap, OBJ mosaic, affine BG | mGBA #2824, #1008, SkyEmu #235 |
| nba-emu hw-test | sprite mosaic, PPU basic timing | mGBA #2933, SkyEmu #116 |
| jsmolka gba-suite (wired, ROM-gated) | CPU, memory, save, ppu hello/shades/stripes | D1 |
| destoer gba_tests `lyc_midline`, `window_midframe` | VCOUNT IRQ on DISPSTAT write, window start/end | mGBA #1873, #1945 |
| butano `issues/` tests (`max_sprites`, `mosaic-test`) | OBJ cycle limit, 256-colour BG mosaic | mGBA #3081, #1684 |
| goodboy-blend-demo, `blending_test` | semi-transparent OBJ vs brightness | mGBA #2722, #3804 |
| gbastuff `dmacnttest`, `dma_mode3` | DMA source control, length | mGBA #2410, #1344 |
| Gericom `directaudiotest` | FIFO size, DMA request timing | mGBA #1847 |
| AGS aging cartridge | broad hardware checks | not free; no harness (`homebrew_suites.rs:22`) |

No GBA-native PSG suite exists; blargg `dmg_sound` tests GB hardware only. For
the APU the practical gate is a headless audio diff against mGBA (how #112 went
from correlation 0.665 to 0.980), plus unit tests in `core/tests/apu.rs`.

## 6. Rules going forward

1. **A class bug gets a `core/tests/` case before any game-specific code.** The
   test reproduces the hardware rule with a hand-built register setup, fails
   first, then the fix lands. No per-game-code workaround for something a class
   test can pin.
2. **An HLE SWI that is not implemented logs once.** `log::warn!` with the SWI
   number and caller PC, the first time each number is seen; the bool from
   `handle_swi` is not discarded. A stub that only approximates (BitUnPack,
   SoundBias) logs the same way until it is real.
3. **Every entry in a gaps list carries an owner, a priority and a
   games-affected column.** An entry without them is a note, not a plan. Ticks in
   `ROADMAP.md` name the test that verifies them; a tick without a test is
   treated as unverified (see `.claude/memory.md` on the decoder bugs).
4. A function whose body is a comment or a partial write is a missing feature
   (memory.md, 2026-08-28). Audits grep for them.
5. Before a PPU or APU change is called done, run the matching test ROM from
   section 5 if present locally, or an mGBA screenshot/audio diff.

## 7. Sources

GBATEK (all sections named above): https://problemkaputt.de/gbatek.htm

mGBA issues (https://github.com/mgba-emu/mgba/issues/N): 142, 190, 356, 377,
539, 875, 899, 932, 1008, 1092, 1265, 1279, 1344, 1396, 1432, 1531, 1668, 1684,
1690, 1691, 1711, 1721, 1756, 1810, 1823, 1847, 1873, 1890, 1945, 2105, 2131,
2141, 2391, 2410, 2443, 2489, 2722, 2809, 2824, 2862, 2933, 2947, 3006, 3081,
3086, 3248, 3258, 3364, 3501, 3512, 3619, 3804.

mGBA changelog and source:
- https://raw.githubusercontent.com/mgba-emu/mgba/master/CHANGES
- https://raw.githubusercontent.com/mgba-emu/mgba/master/src/gba/audio.c
- https://raw.githubusercontent.com/mgba-emu/mgba/master/src/gb/audio.c
- https://raw.githubusercontent.com/mgba-emu/mgba/master/src/gba/io.c
- https://raw.githubusercontent.com/mgba-emu/mgba/master/src/gba/bios.c
- https://github.com/mgba-emu/mgba/commit/1636078b34

NanoBoyAdvance:
- https://github.com/nba-emu/NanoBoyAdvance/releases
- https://newreleases.io/project/github/nba-emu/NanoBoyAdvance/release/v1.8.0
- https://codeberg.org/nba-emu/NanoBoyAdvance/issues/101 , /389 , /125 , /64 , /362
- https://raw.githubusercontent.com/nba-emu/NanoBoyAdvance/master/src/nba/src/hw/apu/apu.cc
- https://raw.githubusercontent.com/nba-emu/NanoBoyAdvance/master/src/nba/src/hw/apu/channel/sweep.hh
- https://raw.githubusercontent.com/nba-emu/NanoBoyAdvance/master/src/nba/src/hw/apu/channel/quad_channel.cc
- https://raw.githubusercontent.com/nba-emu/NanoBoyAdvance/master/src/nba/src/hw/apu/channel/wave_channel.cc

SkyEmu (https://github.com/skylersaleh/SkyEmu/issues/N): 53, 61, 89, 116, 183,
235, 315, 320, 474.

gpSP and Lemuroid:
- https://github.com/libretro/gpsp/issues/224 , /275 , /277
- https://github.com/andymcca/lr-gpsp-amcc
- https://github.com/libretro/mgba/issues/136
- https://github.com/Swordfish90/Lemuroid/issues/815 , /1153

Game code: https://raw.githubusercontent.com/pret/pokeemerald/master/src/m4a.c

Internal: `temp/bug-research/{A,B,C,D1,D2}*.md` (not committed), `ROADMAP.md`
Known accuracy gaps, `.claude/memory.md`, PRs #112 and #113.

### Spot-check (2026-10-27)

Eight URLs were re-fetched while writing this doc. All eight back the claim made
here, so nothing was dropped:

- mGBA #1265: confirms the FR/LG noise super-sampling, "incorrect for GBA audio", Metroid ZM.
- mGBA #1847: confirms the 7-word ring and directaudiotest. Reset-on-overflow is the author's observation, as section 4 says.
- mGBA #3086: confirms the Emerald potion heal sound and the "8-bit write behavior" fix commit. The page does not describe the mechanism.
- mGBA #1721: confirms it is RTC/day-night. The maintainer blames the hack's day/night code, not the clock, which is why it is cited only for bit order.
- SkyEmu #315: confirms Minish Cap and "a bad read mask on SOUNDCNT_X".
- mGBA #2824: confirms the 128 px double-size OBJ wrap, GBATEK quote, and the Tonc `obj_aff` repro.
- mGBA #3248: confirms Radical Red's "BIOS call that is not implemented" message.
- mGBA #3619: confirms "my latching explanation was wrong after all. NBA matches hardware".

Rows the lanes marked INFERRED (mechanism taken from a commit title only) are
used here only as game names and class counts, never as a fix recipe.
