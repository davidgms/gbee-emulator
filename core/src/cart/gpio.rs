//! The cartridge GPIO port and the Seiko S-3511 real-time clock behind it.
//!
//! GBATEK, "GBA Cart I/O Port (GPIO)": three registers in the ROM area,
//! 0x080000C4 data (bits 0-3), 0x080000C6 direction (1 = the GBA drives the
//! pin) and 0x080000C8 control (bit 0: 0 = write-only, the ROM shows through
//! on reads; 1 = readable). GBATEK, "GBA Cart Real-Time Clock (RTC)" and "DS
//! Real-Time Clock": the RTC sits on pin 0 (SCK), 1 (SIO) and 2 (CS). CS high
//! opens a transfer, SIO is sampled on SCK's rising edge and the RTC drives
//! SIO on the falling edge. Bits travel LSB first, the command byte too: in
//! that order its low nibble is the fixed code 6, bits 4-6 the command and
//! bit 7 the read flag (mGBA #1721 was this bit order). Games send the
//! command MSB first as `0x6n`, which is the same byte reversed.
//!
//! Only the RTC is modelled; the solar sensor, gyro and rumble other GPIO
//! carts carry are not.

/// Bytes of [`Gpio::state`] in a save state.
pub(crate) const STATE_LEN: usize = 20;

/// Parameter bytes of each command, indexed by bits 4-6 of the command byte.
/// 0 force reset, 2 date and time, 3 force IRQ, 4 control, 6 time; the rest
/// read as 0xFF (GBATEK: "always FFh").
const PARAM_BYTES: [u8; 8] = [0, 1, 7, 0, 1, 1, 3, 1];

/// Control register bits a write can set: 1 (IRQ duty), 3 (per-minute IRQ),
/// 5 (unknown) and 6 (24-hour mode). Bit 7, power failure, is read-only.
const CONTROL_WRITABLE: u8 = 0x6A;

pub(crate) struct Gpio {
    /// What the GBA last wrote to the data register.
    data: u8,
    direction: u8,
    readable: bool,
    /// The RTC's control register. 0x40 (24-hour mode, power good) is what a
    /// cart the game has already set up holds; 0 would send Emerald through
    /// its reset path at every boot.
    control: u8,
    /// SCK at the previous pin update, for edge detection.
    sck: bool,
    /// SIO as last seen with SCK low: the bit a rising edge shifts in.
    sio_in: bool,
    /// What the RTC drives on SIO.
    sio_out: bool,
    /// The byte being shifted in or out, and how many of its bits have gone.
    shift: u8,
    bits: u8,
    /// The command byte once all eight bits have arrived.
    command: Option<u8>,
    /// Parameter bytes of a read, and the one being shifted out.
    buf: [u8; 7],
    index: u8,
}

impl Gpio {
    pub(crate) fn new() -> Self {
        Self {
            data: 0,
            direction: 0,
            readable: false,
            control: 0x40,
            sck: true,
            sio_in: false,
            sio_out: true,
            shift: 0,
            bits: 0,
            command: None,
            buf: [0xFF; 7],
            index: 0,
        }
    }

    /// The console was reset: the port powers down with it, the RTC runs on
    /// its own battery and keeps its control register.
    pub(crate) fn power_cycle(&mut self) {
        *self = Self {
            control: self.control,
            ..Self::new()
        };
    }

    pub(crate) fn readable(&self) -> bool {
        self.readable
    }

    /// A read of 0x080000C4-0x080000C9 while the port is readable.
    pub(crate) fn read(&self, address: u32) -> u8 {
        match address & 0xF {
            0x4 => self.pins(),
            0x6 => self.direction,
            0x8 => u8::from(self.readable),
            _ => 0,
        }
    }

    /// A byte store to the port. Only the low byte of each register holds
    /// bits. `now` is the local time in seconds since 1970, asked for only
    /// when a command needs the clock.
    pub(crate) fn write(&mut self, address: u32, value: u8, now: impl FnOnce() -> i64) {
        match address & 0xF {
            0x4 => self.data = value & 0xF,
            0x6 => self.direction = value & 0xF,
            0x8 => {
                self.readable = value & 1 != 0;
                return;
            }
            _ => return,
        }
        self.clock(now);
    }

    /// The level on each pin: the GBA's where it drives, the RTC's SIO
    /// elsewhere. The RTC holds SCK and CS low.
    fn pins(&self) -> u8 {
        let rtc = u8::from(self.sio_out) << 1;
        (self.data & self.direction) | (rtc & !self.direction)
    }

    fn clock(&mut self, now: impl FnOnce() -> i64) {
        let pins = self.pins();
        let (sck, sio, cs) = (pins & 1 != 0, pins & 2 != 0, pins & 4 != 0);
        if !cs {
            // CS low ends any transfer; the RTC sees SCK as high.
            self.command = None;
            (self.shift, self.bits, self.index) = (0, 0, 0);
            (self.sck, self.sio_out) = (true, true);
            return;
        }
        let reading = self.command.is_some_and(|c| c & 0x80 != 0);
        if !sck {
            self.sio_in = sio;
        }
        if reading && self.sck && !sck {
            // Falling edge: the next bit out.
            let len = PARAM_BYTES[usize::from((self.command.unwrap_or(0) >> 4) & 7)];
            self.sio_out = self.buf[usize::from(self.index)] >> self.bits & 1 != 0;
            self.bits += 1;
            if self.bits == 8 {
                self.bits = 0;
                self.index = (self.index + 1) % len.max(1);
            }
        } else if !reading && !self.sck && sck {
            // Rising edge: the next bit in.
            self.shift |= u8::from(self.sio_in) << self.bits;
            self.bits += 1;
            if self.bits == 8 {
                let byte = std::mem::take(&mut self.shift);
                self.bits = 0;
                match self.command {
                    None => self.begin(byte, now),
                    // A control register write; date and time writes are
                    // dropped, the clock is the host's.
                    Some(c) if (c >> 4) & 7 == 4 => self.control = byte & CONTROL_WRITABLE,
                    Some(_) => {}
                }
            }
        }
        self.sck = sck;
    }

    fn begin(&mut self, command: u8, now: impl FnOnce() -> i64) {
        if command & 0xF != 6 {
            log::warn!("RTC: invalid command byte {command:#04X}");
            return;
        }
        self.command = Some(command);
        self.index = 0;
        self.buf = [0xFF; 7];
        match (command >> 4) & 7 {
            // GBATEK: force reset also sets the date and time to
            // 2000-01-01 00:00:00. Kept on host time on purpose, as mGBA
            // does: the clock here is the frontend's (`Gba::set_rtc_time`),
            // and a reset that moved it would last only until the next ROM
            // load re-sets it. Emerald resets the chip whenever it finds it
            // in 12-hour mode, so following GBATEK would jump its clock to
            // 2000 for that session.
            0 => self.control = 0,
            4 => self.buf[0] = self.control,
            2 => self.buf = self.date_time(now()),
            6 => {
                let time = self.date_time(now());
                self.buf[..3].copy_from_slice(&time[4..]);
            }
            _ => {}
        }
    }

    /// Year, month, day, day of week (0 = Sunday), hour, minute, second, in
    /// BCD. The hour carries the PM flag in bit 7, in 24-hour mode too.
    fn date_time(&self, local: i64) -> [u8; 7] {
        let bcd = |v: i64| (((v / 10) << 4) | (v % 10)) as u8;
        let days = local.div_euclid(86_400);
        let secs = local.rem_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        let weekday = (days + 4).rem_euclid(7); // 1970-01-01 was a Thursday
        let hour = secs / 3600;
        let shown = if self.control & 0x40 != 0 {
            hour
        } else {
            hour % 12
        };
        [
            bcd(y.rem_euclid(100)),
            bcd(m),
            bcd(d),
            weekday as u8,
            bcd(shown) | if hour >= 12 { 0x80 } else { 0 },
            bcd(secs / 60 % 60),
            bcd(secs % 60),
        ]
    }

    pub(crate) fn state(&self) -> [u8; STATE_LEN] {
        let mut s = [0u8; STATE_LEN];
        s[..12].copy_from_slice(&[
            self.data,
            self.direction,
            u8::from(self.readable),
            self.control,
            u8::from(self.sck),
            u8::from(self.sio_in),
            u8::from(self.sio_out),
            self.shift,
            self.bits,
            u8::from(self.command.is_some()),
            self.command.unwrap_or(0),
            self.index,
        ]);
        s[12..19].copy_from_slice(&self.buf);
        s
    }

    pub(crate) fn restore(&mut self, s: &[u8; STATE_LEN]) -> Result<(), &'static str> {
        let flags = [s[2], s[4], s[5], s[6]];
        if s[0] > 0xF || s[1] > 0xF || s[8] > 7 || s[11] > 6 || flags.iter().any(|&b| b > 1) {
            return Err("RTC state");
        }
        if s[9] > 1 || (s[9] == 1 && s[10] & 0xF != 6) {
            return Err("RTC command");
        }
        *self = Self {
            data: s[0],
            direction: s[1],
            readable: s[2] != 0,
            control: s[3],
            sck: s[4] != 0,
            sio_in: s[5] != 0,
            sio_out: s[6] != 0,
            shift: s[7],
            bits: s[8],
            command: (s[9] != 0).then_some(s[10]),
            index: s[11],
            buf: s[12..19].try_into().unwrap(),
        };
        Ok(())
    }
}

/// Year, month and day of a day count from 1970-01-01, proleptic Gregorian
/// (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Whether this ROM's cartridge carries the RTC: Pokemon Ruby, Sapphire and
/// Emerald, Boktai 1-3, Rockman EXE 4.5 and Sennen Kazoku by game code, or
/// any ROM with Seiko's RTC library in it.
pub(crate) fn has_rtc(rom: &[u8]) -> bool {
    const CODES: [&[u8; 3]; 8] = [
        b"AXV", b"AXP", b"BPE", b"U3I", b"U32", b"U33", b"BR4", b"BKA",
    ];
    rom.get(0xAC..0xAF)
        .is_some_and(|code| CODES.iter().any(|c| code == *c))
        || rom.windows(8).any(|w| w == b"SIIRTC_V")
}

#[cfg(test)]
mod tests {
    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(super::civil_from_days(0), (1970, 1, 1));
        assert_eq!(super::civil_from_days(19_860), (2024, 5, 17));
        assert_eq!(super::civil_from_days(11_016), (2000, 2, 29));
    }
}
