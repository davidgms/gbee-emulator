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
