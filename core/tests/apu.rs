//! APU mixing and frame-sequencer tests, driven through the register writes a
//! game makes. No ROM.
//!
//! Pokemon Emerald's music (m4a/MP2K) plays its square, wave and noise
//! instruments on the PSG channels, so the PSG's envelope speed, length,
//! routing and relative loudness are all audible in it.

use gbee_core::apu::{Apu, SAMPLE_RATE};

const CLOCK: u32 = 16_777_216;

/// Cycles in `ms` milliseconds.
fn ms(ms: u32) -> u32 {
    CLOCK / 1000 * ms
}

/// Sound on, with SOUNDCNT_L and SOUNDCNT_H as given.
fn apu_with(soundcnt_l: u16, soundcnt_h: u16) -> Apu {
    let mut apu = Apu::new();
    apu.write_register(0x84, 0x0080);
    apu.write_register(0x80, soundcnt_l);
    apu.write_register(0x82, soundcnt_h);
    apu
}

/// Run `cycles` and return the samples they produced.
fn run(apu: &mut Apu, cycles: u32) -> Vec<f32> {
    apu.clear_buffer();
    let mut left = cycles;
    while left > 0 {
        let step = left.min(64);
        apu.tick(step);
        left -= step;
    }
    apu.samples().to_vec()
}

fn peak_to_peak(s: &[f32]) -> f32 {
    let max = s.iter().copied().fold(f32::MIN, f32::max);
    let min = s.iter().copied().fold(f32::MAX, f32::min);
    max - min
}

/// Every PSG channel on both sides, master volume 7, PSG at 100%.
const ALL_PSG: u16 = 0xFF77;
const PSG_FULL: u16 = 0x0002;

/// Channel 2 at volume 15, 50% duty, 1024 Hz, with `envelope` in bits 8-11
/// and `control` (length enable) OR'd into SOUND2CNT_H.
fn start_square(apu: &mut Apu, envelope: u16, control: u16) {
    apu.write_register(0x68, 0xF080 | envelope);
    apu.write_register(0x6C, 0x8780 | control);
}

/// GBATEK, SOUND1CNT_H: the envelope steps every n/64 s. The frame
/// sequencer ran every 9856 cycles (1702 Hz) for envelope, sweep and length
/// alike, so a decaying note died 26 times too fast.
#[test]
fn the_envelope_steps_at_64_hz() {
    let mut apu = apu_with(ALL_PSG, PSG_FULL);
    start_square(&mut apu, 0x0100, 0); // step time 1, decreasing
    let start = peak_to_peak(&run(&mut apu, ms(5)));
    run(&mut apu, ms(95));
    let later = peak_to_peak(&run(&mut apu, ms(5)));
    // About six steps in 100 ms: 15 -> 9.
    let ratio = later / start;
    assert!(
        (0.45..0.75).contains(&ratio),
        "volume ratio after 100 ms was {ratio}"
    );
}

/// GBATEK: the length counter runs at 256 Hz, so length 0 (64 steps) is
/// 250 ms of sound.
#[test]
fn the_length_counter_runs_at_256_hz() {
    let mut apu = apu_with(ALL_PSG, PSG_FULL);
    start_square(&mut apu, 0, 0x4000);
    run(&mut apu, ms(190));
    assert!(
        peak_to_peak(&run(&mut apu, ms(10))) > 0.0,
        "silent after 200 ms"
    );
    run(&mut apu, ms(90));
    // Below 1e-3 rather than zero: the DC blocker's tail is still settling.
    let rest = peak_to_peak(&run(&mut apu, ms(10)));
    assert!(rest < 1e-3, "still sounding after 300 ms: {rest}");
}

/// GBATEK, SOUNDCNT_L bits 8-15: a channel routed to neither side is not
/// heard. The mixer ignored the register entirely.
#[test]
fn a_psg_channel_routed_nowhere_is_silent() {
    let mut apu = apu_with(0x0077, PSG_FULL);
    start_square(&mut apu, 0, 0);
    assert_eq!(peak_to_peak(&run(&mut apu, ms(10))), 0.0);
}

/// GBATEK, SOUNDCNT_L bits 0-2 and 4-6: the PSG master volume is (n+1)/8.
#[test]
fn the_psg_master_volume_scales_by_an_eighth_per_step() {
    let mut loud = apu_with(0x2277, PSG_FULL);
    start_square(&mut loud, 0, 0);
    let mut quiet = apu_with(0x2200, PSG_FULL);
    start_square(&mut quiet, 0, 0);
    let ratio = peak_to_peak(&run(&mut quiet, ms(10))) / peak_to_peak(&run(&mut loud, ms(10)));
    assert!((ratio - 0.125).abs() < 0.01, "ratio {ratio}");
}

/// GBATEK, SOUNDCNT_H bits 0-1: PSG volume 25%, 50%, 100%.
#[test]
fn soundcnt_h_scales_the_psg() {
    let mut full = apu_with(ALL_PSG, 0x0002);
    start_square(&mut full, 0, 0);
    let mut quarter = apu_with(ALL_PSG, 0x0000);
    start_square(&mut quarter, 0, 0);
    let ratio = peak_to_peak(&run(&mut quarter, ms(10))) / peak_to_peak(&run(&mut full, ms(10)));
    assert!((ratio - 0.25).abs() < 0.01, "ratio {ratio}");
}

/// GBATEK: "Each of the four PSGs can span one QUARTER of the output range".
/// The wave channel at 100% and the noise channel at volume 15 span exactly
/// what a square at volume 15 does; the wave channel was twice as loud.
#[test]
fn wave_and_noise_span_the_same_range_as_a_square() {
    let mut sq = apu_with(ALL_PSG, PSG_FULL);
    start_square(&mut sq, 0, 0);
    let square = peak_to_peak(&run(&mut sq, ms(20)));

    let mut wave = apu_with(ALL_PSG, PSG_FULL);
    // Bank 1 selected for playback, so these writes land in bank 0.
    wave.write_register(0x70, 0x0040);
    for i in 0..8u32 {
        wave.write_register(0x90 + i * 2, if i < 4 { 0xFFFF } else { 0x0000 });
    }
    wave.write_register(0x70, 0x0080); // play bank 0
    wave.write_register(0x72, 0x2000); // 100%
    wave.write_register(0x74, 0x87C0); // 1024 Hz, like the square
    let wave = peak_to_peak(&run(&mut wave, ms(20)));

    let mut noise = apu_with(ALL_PSG, PSG_FULL);
    noise.write_register(0x78, 0xF000);
    noise.write_register(0x7C, 0x8000);
    let noise = peak_to_peak(&run(&mut noise, ms(20)));

    assert!(
        (wave / square - 1.0).abs() < 0.02,
        "wave {wave} vs square {square}"
    );
    assert!(
        (noise / square - 1.0).abs() < 0.02,
        "noise {noise} vs square {square}"
    );
}

/// The frontend plays one channel, so the stereo pair is averaged. Summing
/// them made a game that pans FIFO A right and FIFO B left - m4a's stereo
/// mode - twice as loud as the same music in mono, and clip.
#[test]
fn the_mono_output_averages_the_two_sides() {
    // A right only, B left only, both 50%, both on timer 0.
    let mut panned = apu_with(0, 0x2100);
    // A and B on both sides at 50% - Emerald's default mono mode.
    let mut centred = apu_with(0, 0x3300);
    for apu in [&mut panned, &mut centred] {
        apu.write_fifo_a(64);
        apu.write_fifo_b(64);
        apu.on_timer_overflow(0);
    }
    let panned = run(&mut panned, ms(1))[0];
    let centred = run(&mut centred, ms(1))[0];
    // One FIFO at 50% is 0.5 * 64/128 = 0.25 of full scale.
    assert!((panned - 0.25).abs() < 1e-3, "panned {panned}");
    assert!((centred - 0.5).abs() < 1e-3, "centred {centred}");
    assert_eq!(SAMPLE_RATE, 48_000);
}

/// GBATEK/pandocs: a trigger with the length counter run out reloads it to
/// the full length. The counter stayed expired, so with length enabled a
/// retriggered note was cut off at the next length clock.
#[test]
fn a_retrigger_after_the_length_ran_out_plays_the_full_length() {
    let mut apu = apu_with(ALL_PSG, PSG_FULL);
    apu.write_register(0x68, 0xF0BF); // length 63: one 256 Hz step
    apu.write_register(0x6C, 0xC780);
    run(&mut apu, ms(20));
    apu.write_register(0x6C, 0xC780); // retrigger, length untouched
    run(&mut apu, ms(100));
    assert!(peak_to_peak(&run(&mut apu, ms(10))) > 0.0);
}

/// The GBA's output stage is AC-coupled, and mGBA's blip_buf removes DC the
/// same way. A 12.5% duty square sat at -0.75 of its amplitude here, so every
/// PSG note-on and note-off was a step in the DC level - a thump - and the
/// offset ate into the headroom on one side.
#[test]
fn the_output_carries_no_dc_offset() {
    let mut apu = apu_with(ALL_PSG, PSG_FULL);
    apu.write_register(0x68, 0xF000); // volume 15, 12.5% duty
    apu.write_register(0x6C, 0x8780);
    run(&mut apu, ms(300));
    let s = run(&mut apu, ms(100));
    let mean = s.iter().sum::<f32>() / s.len() as f32;
    assert!(mean.abs() < 0.005, "DC offset {mean}");
    assert!(peak_to_peak(&s) > 0.4, "the tone itself must survive");
}

// ---------------------------------------------------------------------------
// Through the whole machine: timers, DMA and the bus feeding the APU.
// ---------------------------------------------------------------------------

/// A machine running `b .` from a minimal ROM, so frames only move the clock.
fn spinning_gba() -> gbee_core::Gba {
    let mut rom = vec![0u8; 0x200];
    rom[..4].copy_from_slice(&0xEAFF_FFFEu32.to_le_bytes());
    let mut gba = gbee_core::Gba::new();
    gba.load_rom(&rom).expect("ROM should load");
    gba
}

/// Sign changes in a run of samples.
fn sign_flips(s: &[f32]) -> usize {
    s.windows(2)
        .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
        .count()
}

/// DMA sound on FIFO A from timer 0 at one overflow every 700 cycles - 401
/// a frame - with alternating bytes, so each pop flips the output's sign.
fn dma_sound_gba() -> gbee_core::Gba {
    let mut gba = spinning_gba();
    for i in 0..0x800u32 {
        gba.bus
            .write8(0x0200_0000 + i, if i % 2 == 0 { 0x40 } else { 0xC0 });
    }
    gba.bus.write16(0x0400_0100, (0x1_0000 - 700u32) as u16);
    gba.bus.write16(0x0400_0102, 0x0080);
    // Master on; FIFO A at full volume on both sides, timer 0.
    gba.bus.write16(0x0400_0084, 0x0080);
    gba.bus.write16(0x0400_0082, 0x0304);
    // DMA1 -> FIFO A, 32-bit, repeat, sound timing.
    gba.bus.write32(0x0400_00BC, 0x0200_0000);
    gba.bus.write32(0x0400_00C0, 0x0400_00A0);
    gba.bus.write16(0x0400_00C4, 4);
    gba.bus.write16(0x0400_00C6, 0xB600);
    gba.step(); // let the bus hand the writes over
    gba
}

/// GBATEK, DMA Sound: every overflow of the FIFO's timer plays the next
/// byte, halted CPU or not. A halted step ran up to the next PPU event (up
/// to 960 cycles) with `apu.tick` rendering the whole step before the
/// overflows were handed to the FIFO, so every overflow but the last in a
/// step was popped and never heard: at a 700-cycle timer only about half the
/// bytes reached the output. Games spend most of every frame halted in
/// VBlankIntrWait, so this was the grit on every m4a game's music.
#[test]
fn dma_sound_pops_every_overflow_while_the_cpu_is_halted() {
    let mut gba = dma_sound_gba();
    // Halted for the whole frame: IE is zero, so nothing wakes it.
    gba.cpu.halted = true;
    gba.run_frame();
    gba.clear_audio_buffer();
    gba.run_frame();
    let samples = gba.apu_samples().to_vec();
    assert!(gba.cpu.halted, "the CPU woke up; the test needs it halted");

    // 280896 / 700 = 401 overflows a frame, each held for two samples.
    let flips = sign_flips(&samples);
    assert!(
        (395..=407).contains(&flips),
        "{flips} sign changes in a halted frame, expected one per overflow (~401)"
    );
}

/// The same while the CPU runs and an immediate DMA3 stalls it for about
/// 3900 cycles at a time (320 words, 12 cycles each in EWRAM) - five or six
/// overflows in one step. Games copy tiles and
/// OAM with DMA3 every VBlank, so this is the running-CPU face of the bug.
#[test]
fn dma_sound_pops_every_overflow_during_a_long_dma() {
    let mut gba = dma_sound_gba();
    gba.run_frame();
    gba.clear_audio_buffer();
    let end = gba.cycles + 280_896;
    while gba.cycles < end {
        // EWRAM to EWRAM, 320 words, immediate.
        gba.bus.write32(0x0400_00D4, 0x0201_0000);
        gba.bus.write32(0x0400_00D8, 0x0202_0000);
        gba.bus.write16(0x0400_00DC, 320);
        gba.bus.write16(0x0400_00DE, 0x8400);
        gba.step();
    }
    let flips = sign_flips(gba.apu_samples());
    assert!(
        (395..=407).contains(&flips),
        "{flips} sign changes in a frame of long DMAs, expected ~401"
    );
}

/// A slice with more overflows than the FIFO holds: a 2048-word DMA3 stalls
/// the CPU for about 24600 cycles, 35 pops of a FIFO that holds at most 32
/// bytes and is refilled 16 at a time. The refill has to run between the
/// pops, as the hardware's sound DMA request does, not once per slice: the
/// FIFO ran dry halfway through and the second half of every slice held one
/// byte.
#[test]
fn a_long_slice_refills_the_fifo_between_pops() {
    let mut gba = dma_sound_gba();
    gba.run_frame();
    gba.clear_audio_buffer();
    let start = gba.cycles;
    while gba.cycles < start + 280_896 {
        gba.bus.write32(0x0400_00D4, 0x0201_0000);
        gba.bus.write32(0x0400_00D8, 0x0202_0000);
        gba.bus.write16(0x0400_00DC, 2048);
        gba.bus.write16(0x0400_00DE, 0x8400);
        gba.step();
    }
    // The last slice runs past the frame, so count what actually ran.
    let expected = ((gba.cycles - start) / 700) as usize;
    let flips = sign_flips(gba.apu_samples());
    assert!(
        flips.abs_diff(expected) <= 6,
        "{flips} sign changes with 35 pops a slice, expected ~{expected}"
    );
}

/// The registers of a square on channel 2 through the bus: SOUND2CNT_L
/// envelope `envelope`, SOUND2CNT_H frequency 0x780 with the trigger.
fn psg_gba(envelope: u16) -> gbee_core::Gba {
    let mut gba = spinning_gba();
    gba.bus.write16(0x0400_0084, 0x0080);
    gba.bus.write16(0x0400_0080, ALL_PSG);
    gba.bus.write16(0x0400_0082, PSG_FULL);
    gba.bus.write16(0x0400_0068, envelope);
    gba.bus.write16(0x0400_006C, 0x8780);
    gba
}

/// Peak-to-peak of the next `frames` frames of output.
fn frames_peak(gba: &mut gbee_core::Gba, frames: u32) -> f32 {
    gba.clear_audio_buffer();
    for _ in 0..frames {
        gba.run_frame();
    }
    peak_to_peak(gba.apu_samples())
}

/// GBATEK, Sound Channel 1/2: SOUNDxCNT_X/H bit 15 is a write-only "Initial"
/// bit. It used to stay latched in the I/O registers, and every sound byte
/// write re-decoded the whole halfword, so m4a's `strb` to the frequency low
/// byte (vibrato, pitch bends) restarted the note and its envelope. mGBA
/// #3086 and NanoBoyAdvance #389 fixed the same bug (Emerald's heal sound).
#[test]
fn a_frequency_low_byte_write_does_not_retrigger_the_channel() {
    // Volume 15, decreasing one step every 1/64 s: silent after ~235 ms.
    let mut gba = psg_gba(0xF180);
    assert!(frames_peak(&mut gba, 2) > 0.1, "the note never started");
    frames_peak(&mut gba, 20);
    assert!(
        frames_peak(&mut gba, 2) < 1e-3,
        "the envelope never decayed"
    );

    gba.bus.write8(0x0400_006C, 0x90); // new pitch, low byte only
    assert!(
        frames_peak(&mut gba, 2) < 1e-3,
        "a write to the frequency low byte restarted the note"
    );

    // The high byte with bit 7 set is a real trigger.
    gba.bus.write8(0x0400_006D, 0x87);
    assert!(
        frames_peak(&mut gba, 2) > 0.1,
        "a write of the trigger bit no longer restarts the note"
    );
}

/// The same for the wave and noise channels, and the trigger reads back 0.
#[test]
fn the_trigger_bit_is_write_only_on_every_psg_channel() {
    let mut gba = spinning_gba();
    for (reg, value) in [
        (0x0400_0064u32, 0x8700u16),
        (0x0400_006C, 0x8700),
        (0x0400_0074, 0x8700),
        (0x0400_007C, 0x8000),
    ] {
        gba.bus.write16(reg, value);
        gba.step();
        assert_eq!(
            gba.bus.io_read16(reg) & 0x8000,
            0,
            "{reg:#x}: the trigger bit stayed latched"
        );
    }
}
