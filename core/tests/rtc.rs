//! Cartridge GPIO port and the S-3511 real-time clock behind it.
//!
//! GBATEK, "GBA Cart I/O Port (GPIO)" and "GBA Cart Real-Time Clock (RTC)":
//! the port is three registers at 0x080000C4 (data), 0x080000C6 (direction)
//! and 0x080000C8 (control, bit 0 = readable). The RTC hangs off data bits
//! 0 (SCK), 1 (SIO) and 2 (CS) and speaks a 3-wire serial protocol.
//!
//! The helpers bit-bang the port exactly as pokeemerald's `siirtc.c` does:
//! the command byte goes out MSB first, parameter bytes LSB first, and a bit
//! is clocked on SCK's rising edge. Nothing here existed before, so Ruby,
//! Sapphire and Emerald reported "The internal battery has run dry" and ran
//! no clock-based event.

use gbee_core::Gba;

const DATA: u32 = 0x0800_00C4;
const DIR: u32 = 0x0800_00C6;
const CTRL: u32 = 0x0800_00C8;
const SCK: u16 = 1;
const CS: u16 = 4;

/// A ROM with `code` as its game code, `b .` at the entry point and `0x5A`
/// bytes where the GPIO port sits, so a ROM read there is recognisable.
fn rom(code: &[u8; 4]) -> Vec<u8> {
    let mut rom = vec![0u8; 0x400];
    rom[..4].copy_from_slice(&0xEAFF_FFFEu32.to_le_bytes());
    rom[0xAC..0xB0].copy_from_slice(code);
    rom[0xC4..0xCA].fill(0x5A);
    rom
}

fn gba_with(code: &[u8; 4]) -> Gba {
    let mut gba = Gba::new();
    gba.load_rom(&rom(code)).expect("ROM should load");
    gba
}

/// Pokemon Emerald's game code; its cart carries the RTC.
fn emerald() -> Gba {
    gba_with(b"BPEE")
}

/// CS high with SCK high, every pin an output, the port readable.
fn begin(gba: &mut Gba) {
    gba.bus.write16(CTRL, 1);
    gba.bus.write16(DATA, SCK);
    gba.bus.write16(DATA, SCK | CS);
    gba.bus.write16(DIR, 7);
}

fn end(gba: &mut Gba) {
    gba.bus.write16(DATA, SCK);
}

fn clock_out(gba: &mut Gba, bit: u16) {
    gba.bus.write16(DATA, (bit << 1) | CS);
    gba.bus.write16(DATA, (bit << 1) | CS);
    gba.bus.write16(DATA, (bit << 1) | CS | SCK);
}

fn send_command(gba: &mut Gba, command: u8) {
    for i in (0..8).rev() {
        clock_out(gba, u16::from(command >> i) & 1);
    }
}

fn send_byte(gba: &mut Gba, value: u8) {
    for i in 0..8 {
        clock_out(gba, u16::from(value >> i) & 1);
    }
}

fn read_byte(gba: &mut Gba) -> u8 {
    gba.bus.write16(DIR, 5); // SIO is an input now
    let mut value = 0;
    for i in 0..8 {
        gba.bus.write16(DATA, CS);
        gba.bus.write16(DATA, CS | SCK);
        value |= ((gba.bus.read16(DATA) >> 1) & 1) as u8 * (1 << i);
    }
    value
}

/// One whole command: `command`, then `n` bytes read back.
fn read_command(gba: &mut Gba, command: u8, n: usize) -> Vec<u8> {
    begin(gba);
    send_command(gba, command);
    let bytes = (0..n).map(|_| read_byte(gba)).collect();
    end(gba);
    bytes
}

fn write_command(gba: &mut Gba, command: u8, bytes: &[u8]) {
    begin(gba);
    send_command(gba, command);
    for &b in bytes {
        send_byte(gba, b);
    }
    end(gba);
}

/// Seconds since 1970-01-01 of a local date and time, for `set_rtc_time`.
fn local_seconds(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> i64 {
    // Days from civil, proleptic Gregorian (Howard Hinnant's algorithm).
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((mo + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400 + h * 3600 + mi * 60 + s
}

/// GBATEK: command 0x63 reads the control register. Emerald's
/// `SiiRtcProbe` sends exactly this and wants 24-hour mode (bit 6) without
/// the power-failure flag (bit 7); a cart a game has set up reads 0x40.
#[test]
fn an_rtc_cart_answers_the_gpio_status_command() {
    let mut gba = emerald();
    assert_eq!(read_command(&mut gba, 0x63, 1), [0x40]);
}

/// Reset (0x60) zeroes the control register; a write (0x62) sets it.
#[test]
fn the_rtc_control_register_resets_and_takes_writes() {
    let mut gba = emerald();
    write_command(&mut gba, 0x60, &[]);
    assert_eq!(read_command(&mut gba, 0x63, 1), [0x00]);
    write_command(&mut gba, 0x62, &[0x40]);
    assert_eq!(read_command(&mut gba, 0x63, 1), [0x40]);
}

/// Date and time (0x65) is seven BCD bytes: year, month, day, day of week,
/// hour (bit 7 = PM, set in 24-hour mode too, which is why pokeemerald masks
/// it off), minute, second. Time (0x67) is the last three.
#[test]
fn an_rtc_cart_reads_the_date_and_time_in_bcd() {
    let mut gba = emerald();
    // A Friday afternoon.
    gba.set_rtc_time(local_seconds(2024, 5, 17, 13, 45, 30));
    let dt = read_command(&mut gba, 0x65, 7);
    assert_eq!(dt[..6], [0x24, 0x05, 0x17, 5, 0x80 | 0x13, 0x45]);
    assert!(matches!(dt[6], 0x30 | 0x31), "seconds {:#x}", dt[6]);

    let t = read_command(&mut gba, 0x67, 3);
    assert_eq!(t[..2], [0x93, 0x45]);

    // 12-hour mode: 13:45 is 01:45 PM.
    write_command(&mut gba, 0x62, &[0x00]);
    assert_eq!(read_command(&mut gba, 0x67, 3)[..2], [0x81, 0x45]);
}

/// GBATEK: in write-only mode (control bit 0 clear) a read of the port
/// returns the ROM underneath; a cart with no RTC never answers at all.
#[test]
fn the_gpio_port_reads_rom_unless_enabled_on_an_rtc_cart() {
    let mut gba = emerald();
    assert_eq!(gba.bus.read16(DATA), 0x5A5A, "write-only by default");
    gba.bus.write16(CTRL, 1);
    assert_eq!(gba.bus.read16(CTRL), 1, "readable once enabled");
    assert_eq!(gba.bus.read32(0x0800_00C0) & 0xFFFF, 0, "ROM below it");

    let mut plain = gba_with(b"AGBE");
    plain.bus.write16(CTRL, 1);
    assert_eq!(plain.bus.read16(DATA), 0x5A5A, "no RTC: always ROM");
}

/// The RTC's registers are battery-backed chip state, and a save state taken
/// on an RTC cart carries them (state v12).
#[test]
fn a_save_state_carries_the_rtc_registers() {
    let mut gba = emerald();
    write_command(&mut gba, 0x60, &[]); // control register now 0
    let state = gba.save_state();

    let mut other = emerald();
    other.load_state(&state).expect("state should load");
    assert_eq!(read_command(&mut other, 0x63, 1), [0x00]);
}
