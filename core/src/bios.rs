// GBA BIOS HLE (High-Level Emulation)
// Handles common SWI calls without requiring a real BIOS ROM.

use std::sync::atomic::{AtomicU64, Ordering};

use super::cpu::Cpu;
use super::memory::MemoryBus;

/// Handle a BIOS SWI call. Returns true if handled.
pub fn handle_swi(swi_num: u32, cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    match swi_num {
        0x00 => handle_soft_reset(cpu, bus),
        0x01 => handle_register_ram_reset(cpu, bus),
        0x02 => handle_halt(cpu, bus),
        0x04 => handle_intr_wait(cpu, bus),
        0x05 => handle_vblank_intr_wait(cpu, bus),
        0x06 => handle_div(cpu),
        0x07 => handle_div_arm(cpu),
        0x08 => handle_sqrt(cpu),
        0x09 => handle_arc_tan(cpu),
        0x0A => handle_arc_tan2(cpu),
        0x0B => handle_cpu_set(cpu, bus),
        0x0C => handle_cpu_fast_set(cpu, bus),
        0x0D => handle_get_bios_checksum(cpu),
        0x0E => handle_bg_affine_set(cpu, bus),
        0x0F => handle_obj_affine_set(cpu, bus),
        0x10 => handle_bit_unpack(cpu, bus),
        0x11 => handle_lz77_uncomp_wram(cpu, bus),
        0x12 => handle_lz77_uncomp_vram(cpu, bus),
        0x13 => handle_huff_uncomp(cpu, bus),
        0x14 => handle_rl_uncomp_wram(cpu, bus),
        0x15 => handle_rl_uncomp_vram(cpu, bus),
        0x16 => handle_diff8bit_unfilter_wram(cpu, bus),
        0x17 => handle_diff8bit_unfilter_vram(cpu, bus),
        0x18 => handle_diff16bit_unfilter(cpu, bus),
        0x19 => handle_sound_bias(cpu, bus),
        0x1F => handle_midi_key2freq(cpu, bus),
        _ => {
            report_unhandled(swi_num);
            false
        }
    }
}

/// One bit per SWI number already reported by [`report_unhandled`].
static REPORTED: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];

/// Warn about an SWI the HLE does not implement, once per number for the
/// life of the process: a game calling it every frame must not flood the log.
/// It used to return silently with r0-r3 unchanged, so a game depending on it
/// failed somewhere else and the failure looked like a game bug.
fn report_unhandled(swi_num: u32) {
    let n = (swi_num & 0xFF) as usize;
    let bit = 1u64 << (n % 64);
    if REPORTED[n / 64].fetch_or(bit, Ordering::Relaxed) & bit == 0 {
        log::warn!("unimplemented BIOS call: SWI 0x{n:02X}, returning with registers unchanged");
    }
}

/// The BIOS IRQ handler ORs the interrupts it acknowledged into this
/// halfword, and `IntrWait` consumes them from it.
const BIOS_INTR_FLAGS: u32 = 0x0300_7FF8;

/// SWI 0x00. GBATEK, BIOS Reset Functions: clears 0x3007E00-0x3007FFF,
/// sets SP_svc/SP_irq/SP_sys, zeroes R0-R12, LR_svc, SPSR_svc, LR_irq and
/// SPSR_irq, enters System mode and `BX R14`s to 0x08000000 - or to
/// 0x02000000 if the byte at 0x3007FFA, read before the clear, is non-zero.
fn handle_soft_reset(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let target = if bus.read8(0x0300_7FFA) != 0 {
        0x0200_0000
    } else {
        0x0800_0000
    };
    bus.iwram_data_mut()[0x7E00..].fill(0);

    // System mode, ARM state; set_cpsr parks the caller's bank first.
    cpu.set_cpsr(0x1F);
    cpu.svc_registers = [0x0300_7FE0, 0];
    cpu.irq_registers = [0x0300_7FA0, 0];
    cpu.spsr_svc = 0;
    cpu.spsr_irq = 0;
    cpu.registers[..13].fill(0);
    cpu.registers[13] = 0x0300_7F00;
    cpu.registers[14] = target;
    cpu.set_reg(15, target);
    true
}

/// SWI 0x01. GBATEK, BIOS Reset Functions: bit 0 clears EWRAM, bit 1 IWRAM
/// "excluding last 200h bytes" (the stack, the IRQ vector and the BIOS flags
/// live there), bits 2-4 palette, VRAM and OAM, bit 5 resets the SIO
/// registers, bit 6 the sound registers, bit 7 every other register. The
/// screen is forced blank (DISPCNT = 0x0080) whatever r0 holds.
///
/// Registers go through the bus as a CPU store would, so the APU, DMA and
/// timer queues see them. The register lists follow mGBA's `bios.c`.
fn handle_register_ram_reset(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let flags = cpu.reg(0);
    bus.write16(0x0400_0000, 0x0080);
    if flags & 0x01 != 0 {
        bus.ewram_data_mut().fill(0);
    }
    if flags & 0x02 != 0 {
        bus.iwram_data_mut()[..0x7E00].fill(0);
    }
    if flags & 0x04 != 0 {
        bus.palette_data_mut().fill(0);
    }
    if flags & 0x08 != 0 {
        bus.vram_data_mut().fill(0);
    }
    if flags & 0x10 != 0 {
        bus.oam_data_mut().fill(0);
    }
    let zero = |bus: &mut MemoryBus, from: u32, to: u32| {
        for reg in (from..to).step_by(2) {
            bus.write16(0x0400_0000 + reg, 0);
        }
    };
    if flags & 0x20 != 0 {
        bus.write16(0x0400_0128, 0); // SIOCNT
        bus.write16(0x0400_0134, 0x8000); // RCNT: general purpose
        bus.write16(0x0400_012A, 0); // SIOMLT_SEND
        bus.write16(0x0400_0140, 0); // JOYCNT
        zero(bus, 0x150, 0x158); // JOY_RECV, JOY_TRANS
    }
    if flags & 0x40 != 0 {
        // SOUNDCNT_X last, as the master enable; wave RAM too.
        zero(bus, 0x60, 0x84);
        zero(bus, 0x90, 0xA0);
        bus.write16(0x0400_0084, 0);
        bus.write16(0x0400_0088, 0x0200); // SOUNDBIAS
    }
    if flags & 0x80 != 0 {
        zero(bus, 0x04, 0x56); // DISPSTAT to BLDY
        for pa_pd in [0x20, 0x26, 0x30, 0x36] {
            bus.write16(0x0400_0000 + pa_pd, 0x0100);
        }
        zero(bus, 0xB0, 0xE0); // DMA
        zero(bus, 0x100, 0x110); // timers
        bus.write16(0x0400_0200, 0); // IE
        bus.write16(0x0400_0202, 0xFFFF); // IF, acknowledged
        bus.write16(0x0400_0204, 0); // WAITCNT
        bus.write16(0x0400_0208, 0); // IME
    }
    true
}

/// SWI 0x02: Halt - enters low-power state until interrupt
fn handle_halt(_cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    bus.io.halt = true;
    bus.io.halt_swi = true;
    true
}

/// Cycles the BIOS spends between an IRQ handler returning into `IntrWait`'s
/// halt loop and the caller's next instruction, beyond the 3 this HLE charges
/// for re-running the SWI.
///
/// The real BIOS runs code there: the flag check (IME off, `ldrh`, `ands`,
/// `eorne`, `strneh`, IME on, `beq`) and the SWI dispatcher's epilogue
/// (`ldmfd`, `msr`, `ldmfd`, `msr`, `ldmfd`, `movs pc, lr`). mGBA's
/// cycle-padded HLE BIOS tail (`src/gba/hle-bios.s`, from the halt loop's `1:`
/// to `movs pc, lr`) costs 39 cycles on this CPU; the real BIOS calls its
/// check as a subroutine, which adds a `bl`/`bx lr` pair. The value is fitted
/// to the mGBA suite's count-up test, whose results were recorded on hardware
/// and resolve it to the cycle: 44 fails 16 of 936 checks, 45 none, 46 18
/// (`timer_count_up_matches_the_mgba_suite`).
const INTR_WAIT_RETURN_CYCLES: u32 = 45;

/// Cycles the BIOS spends after a wake from `Halt` (SWI 02h) before the
/// caller's next instruction: the `bx lr` out of the halt routine and the SWI
/// dispatcher's epilogue (`ldmfd`, `msr`, `ldmfd`, `msr`, `ldmfd`,
/// `movs pc, lr`), which this HLE does not execute. Same idea as
/// `INTR_WAIT_RETURN_CYCLES`, without its flag check. Fitted to the mGBA
/// suite's SIO timing test (hardware values): its four normal-mode rows all
/// sit 30 cycles above the bit time plus this core's IRQ path, and 29 or 31
/// fail all four. That test cannot tell a halt return cost from a transfer
/// start latency; the halt return is the one known to be missing here.
pub(crate) const HALT_RETURN_CYCLES: u32 = 30;

/// SWI 0x04: IntrWait(r1=discardOldFlags, r2=IEFlags)
/// SWI 04h. GBATEK: "Continues to wait in Halt state until one (or more) of
/// the specified interrupt(s) do occur. **The function forcefully sets
/// IME=1.**"
fn handle_intr_wait(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let discard = cpu.reg(0) != 0;
    let wanted = (cpu.reg(1) & 0xFFFF) as u16;

    // Forcing IME is not optional: a game may call this with interrupts
    // globally masked and rely on the BIOS to enable them.
    bus.io.ime = 1;

    // The BIOS does not watch IF. Its IRQ handler ORs whatever it acknowledged
    // into the halfword at 0x03007FF8 and IntrWait polls *that*, because IF is
    // already cleared by the time the game's handler returns. Waiting on IF
    // instead meant the first interrupt of any kind - an HBlank, typically -
    // satisfied a VBlankIntrWait.
    if discard && !bus.io.intr_wait_active {
        let flags = bus.read16(BIOS_INTR_FLAGS);
        bus.write16(BIOS_INTR_FLAGS, flags & !wanted);
    }

    let flags = bus.read16(BIOS_INTR_FLAGS);
    if flags & wanted != 0 {
        bus.write16(BIOS_INTR_FLAGS, flags & !wanted);
        if bus.io.intr_wait_active {
            cpu.entry_cycles += INTR_WAIT_RETURN_CYCLES;
        }
        bus.io.intr_wait_active = false;
        return true;
    }

    // Not satisfied: halt, and rewind onto this SWI so it runs again when an
    // interrupt wakes us. That is the BIOS's own loop, not an approximation
    // of it, so a wait for VBlank keeps waiting through every other source.
    bus.io.intr_wait_active = true;
    bus.io.halt = true;
    let pipeline = if cpu.cpsr & 0x20 != 0 { 4 } else { 8 };
    let here = cpu.registers[15].wrapping_sub(pipeline);
    cpu.set_reg(15, here);
    true
}

/// SWI 0x05: VBlankIntrWait - waits specifically for VBlank
/// SWI 05h. GBATEK: "sets r0=1, r1=1, and then calls IntrWait".
fn handle_vblank_intr_wait(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    cpu.set_reg(0, 1); // discard old flags
    cpu.set_reg(1, 0x0001); // wait for VBlank
    handle_intr_wait(cpu, bus)
}

/// SWI 06h: Div(r0 = numerator, r1 = denominator).
fn handle_div(cpu: &mut Cpu) -> bool {
    div(cpu, cpu.reg(0) as i32, cpu.reg(1) as i32)
}

/// SWI 07h: DivArm, the same with the operands swapped.
fn handle_div_arm(cpu: &mut Cpu) -> bool {
    div(cpu, cpu.reg(1) as i32, cpu.reg(0) as i32)
}

/// GBATEK, Div: r0 = number DIV denom, r1 = number MOD denom, r3 =
/// ABS(number DIV denom). GBATEK leaves the edge cases open; the mGBA suite's
/// hardware table settles them:
/// - INT_MIN / -1 wraps: r0 = r3 = 0x80000000, r1 = 0.
/// - x / 0 for x in {0, 1, -1} returns r0 = 1, 1, -1, with r1 = x and r3 = 1.
///   For larger |x| the real BIOS never returns; the HLE extends the observed
///   pattern (sign of x, never 0) instead of hanging the machine.
fn div(cpu: &mut Cpu, numerator: i32, denominator: i32) -> bool {
    let (quotient, remainder) = if denominator == 0 {
        (numerator.signum() | 1, numerator)
    } else {
        // `wrapping_*`: Rust's `/` panics on INT_MIN / -1 even in release,
        // which would abort the whole process through JNI.
        (
            numerator.wrapping_div(denominator),
            numerator.wrapping_rem(denominator),
        )
    };
    cpu.set_reg(0, quotient as u32);
    cpu.set_reg(1, remainder as u32);
    cpu.set_reg(3, quotient.wrapping_abs() as u32);
    true
}

fn handle_sqrt(cpu: &mut Cpu) -> bool {
    let input = cpu.reg(0);
    let result = (input as f64).sqrt() as u32;
    cpu.set_reg(0, result);
    true
}

/// The BIOS ArcTan polynomial's constants, highest order first, 1.14 fixed
/// point. GBATEK documents the interface only; these are constants of the
/// BIOS ROM's routine, and every ArcTan/ArcTan2 row of the mGBA suite's
/// hardware table (`bios_math_matches_the_mgba_suite_hardware_table`) checks
/// them - changing any one of them fails rows.
const ARCTAN_COEFFICIENTS: [i32; 8] = [0xA9, 0x390, 0x91C, 0xFB6, 0x16AA, 0x2081, 0x3651, 0xA2F9];

/// The arc tangent of a 1.14 fixed-point `tan`, as `(angle, a, b)`.
///
/// GBATEK, ArcTan: the result covers -PI/2..PI/2 as C000h..4000h. The BIOS
/// evaluates an odd polynomial: `a = -(tan^2)`, `b` = the coefficients in
/// Horner form over `a`, angle = `tan * b >> 16`. `a` and `b` are what the
/// hardware table shows left in r1 and r3. Everything is 32-bit wrapping, as
/// ARM code computes it, and the whole of r0 is the input (the table reads
/// 0x0000C000 as +3.0, not -1.0).
fn arc_tan(tan: i32) -> (i32, i32, i32) {
    let a = (tan.wrapping_mul(tan) >> 14).wrapping_neg();
    let b = ARCTAN_COEFFICIENTS[1..]
        .iter()
        .fold(ARCTAN_COEFFICIENTS[0], |b, &c| {
            (b.wrapping_mul(a) >> 14).wrapping_add(c)
        });
    (tan.wrapping_mul(b) >> 16, a, b)
}

/// SWI 09h: ArcTan(r0 = tan) -> r0 = angle (sign-extended halfword),
/// r1 = a, r3 = b.
fn handle_arc_tan(cpu: &mut Cpu) -> bool {
    let (angle, a, b) = arc_tan(cpu.reg(0) as i32);
    cpu.set_reg(0, angle as i16 as u32);
    cpu.set_reg(1, a as u32);
    cpu.set_reg(3, b as u32);
    true
}

/// SWI 0Ah: ArcTan2(r0 = x, r1 = y) -> r0 = 0000h..FFFFh for 0 <= THETA < 2PI
/// (GBATEK). r1 is the `a` of the series call, left alone (= y) when the point
/// lies on an axis; r3 is 0x170, a BIOS address, in every row of the
/// hardware table.
fn handle_arc_tan2(cpu: &mut Cpu) -> bool {
    let x = cpu.reg(0) as i32;
    let y = cpu.reg(1) as i32;
    let (angle, a) = arc_tan2(x, y);
    cpu.set_reg(0, angle);
    if let Some(a) = a {
        cpu.set_reg(1, a);
    }
    cpu.set_reg(3, 0x170);
    true
}

/// The series is only accurate for |tan| <= 1, so the angle is measured from
/// the nearer axis: from the x axis with tan = y/x when |y| <= |x|, otherwise
/// from the y axis with tan = x/y, which runs the other way and is
/// subtracted. The series result is a halfword; the sum wraps to 16 bits.
fn arc_tan2(x: i32, y: i32) -> (u32, Option<u32>) {
    if y == 0 {
        return (if x >= 0 { 0 } else { 0x8000 }, None);
    }
    if x == 0 {
        return (if y > 0 { 0x4000 } else { 0xC000 }, None);
    }
    let series = |n: i32, d: i32| {
        let (angle, a, _) = arc_tan((n << 14).wrapping_div(d));
        (i32::from(angle as i16), a)
    };
    let (angle, a) = if y.unsigned_abs() <= x.unsigned_abs() {
        let (angle, a) = series(y, x);
        (if x > 0 { 0 } else { 0x8000 } + angle, a)
    } else {
        let (angle, a) = series(x, y);
        (if y > 0 { 0x4000 } else { 0xC000 } - angle, a)
    };
    ((angle as u32) & 0xFFFF, Some(a as u32))
}

/// CpuSet and CpuFastSet copy nothing from below EWRAM: the BIOS protects
/// its own ROM, and the unmapped space after it goes with it (mGBA suite,
/// Memory "swi B"/"swi C" expect the destination untouched; mGBA's HLE
/// refuses the same range).
const BIOS_COPY_MIN_SOURCE: u32 = 0x0200_0000;

/// SWI 0x0B: CpuSet(r0 = src, r1 = dst, r2 = length/mode).
///
/// GBATEK, BIOS Memory Copy: bits 0-20 are the (half)word count, bit 24 fixes
/// the source address (0 = copy, 1 = fill), bit 26 is the unit size
/// (0 = 16bit, 1 = 32bit).
fn handle_cpu_set(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let cnt = cpu.reg(2);
    if src < BIOS_COPY_MIN_SOURCE {
        return true;
    }
    let count = cnt & 0x001F_FFFF;
    let fill = cnt & 0x0100_0000 != 0;
    let is_32bit = cnt & 0x0400_0000 != 0;
    if is_32bit {
        for i in 0..count {
            let src_addr = if fill { src } else { src.wrapping_add(i * 4) };
            let val = bus.read32(src_addr);
            bus.write32(dst.wrapping_add(i * 4), val);
        }
    } else {
        for i in 0..count {
            let src_addr = if fill { src } else { src.wrapping_add(i * 2) };
            // The BIOS loop is `ldrh`, which rotates an odd address's
            // halfword right by 8; only the low half reaches `strh`.
            let val = bus.read16(src_addr) >> ((src_addr & 1) * 8);
            bus.write16(dst.wrapping_add(i * 2), val);
        }
    }
    true
}

/// SWI 0x0C: CpuFastSet(r0 = src, r1 = dst, r2 = length/mode).
///
/// GBATEK, BIOS Memory Copy: word-only transfers in units of 32 bytes. Bits
/// 0-20 are the word count, "GBA: rounded-up to multiple of 8 words", and bit
/// 24 fixes the source address (0 = copy, 1 = fill by WORD[r0]).
fn handle_cpu_fast_set(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let cnt = cpu.reg(2);
    if src < BIOS_COPY_MIN_SOURCE {
        return true;
    }
    let count = (cnt & 0x001F_FFFF).wrapping_add(7) & !7;
    let fill = cnt & 0x0100_0000 != 0;
    for i in 0..count {
        let src_addr = if fill { src } else { src.wrapping_add(i * 4) };
        let val = bus.read32(src_addr);
        bus.write32(dst.wrapping_add(i * 4), val);
    }
    true
}

/// Most entries one BgAffineSet / ObjAffineSet call processes.
///
/// r2 is a full guest u32, and four billion entries kept the emulation thread
/// inside one SWI for minutes. Real calls fill one matrix per background or
/// sprite, or at most one per scanline for an HBlank DMA table (228). 65536
/// BgAffineSet entries are 1 MB of output, four times all of EWRAM, so no
/// call a game can make use of is cut short.
const AFFINE_SET_MAX: u32 = 0x1_0000;

/// `sin(angle)` in 1.14 fixed point, for the upper byte of a BIOS angle
/// (0x00-0xFF = one full turn).
///
/// The real BIOS reads a 256-entry table instead. Rounding the f64 sine is
/// not checked against a BIOS dump and may be one LSB off on some entries
/// (1/256 px on an affine sprite); 0x40 lands on 0x4000.
fn bios_sin(index: u32) -> i32 {
    let turn = f64::from(index & 0xFF) / 256.0 * std::f64::consts::TAU;
    (turn.sin() * 16384.0).round() as i32
}

/// The affine matrix both affine sets share, in 8.8 fixed point:
/// pa = sx*cos, pb = -sx*sin, pc = sy*sin, pd = sy*cos (GBATEK, BIOS
/// Rotation/Scaling Functions). Only the upper byte of the angle is used.
///
/// Products are shifted arithmetically (the BIOS's `asr #14`) and pb is
/// negated after the shift. mGBA computes the same matrix in float and
/// truncates, which differs from this by one in the last bit on some
/// negative entries.
fn affine_matrix(sx: i32, sy: i32, angle: u16) -> [i32; 4] {
    let step = u32::from(angle >> 8);
    let (sin, cos) = (bios_sin(step), bios_sin(step + 0x40));
    [
        (sx * cos) >> 14,
        -((sx * sin) >> 14),
        (sy * sin) >> 14,
        (sy * cos) >> 14,
    ]
}

/// SWI 0x0E: BgAffineSet(r0 = src, r1 = dst, r2 = count).
///
/// GBATEK: each 20-byte source entry is s32 ox, s32 oy (8.8, texture
/// centre), s16 cx, s16 cy (screen centre), s16 sx, s16 sy (8.8 scale) and a
/// u16 angle; each 16-byte destination entry is pa, pb, pc, pd and the s32
/// start point x = ox - (pa*cx + pb*cy), y = oy - (pc*cx + pd*cy).
fn handle_bg_affine_set(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let count = cpu.reg(2).min(AFFINE_SET_MAX);
    for i in 0..count {
        let src_off = src.wrapping_add(i.wrapping_mul(20));
        let dst_off = dst.wrapping_add(i.wrapping_mul(16));
        let half = |o: u32| i32::from(bus.read16(src_off.wrapping_add(o)) as i16);
        let ox = bus.read32(src_off) as i32;
        let oy = bus.read32(src_off.wrapping_add(4)) as i32;
        let (cx, cy) = (half(8), half(10));
        let m = affine_matrix(half(12), half(14), bus.read16(src_off.wrapping_add(16)));
        for (k, &v) in m.iter().enumerate() {
            bus.write16(dst_off.wrapping_add(k as u32 * 2), v as u16);
        }
        let x = ox.wrapping_sub(m[0].wrapping_mul(cx).wrapping_add(m[1].wrapping_mul(cy)));
        let y = oy.wrapping_sub(m[2].wrapping_mul(cx).wrapping_add(m[3].wrapping_mul(cy)));
        bus.write32(dst_off.wrapping_add(8), x as u32);
        bus.write32(dst_off.wrapping_add(12), y as u32);
    }
    true
}

/// SWI 0x0F: ObjAffineSet(r0 = src, r1 = dst, r2 = count, r3 = stride).
///
/// GBATEK: each 8-byte source entry is s16 sx, s16 sy (8.8), u16 angle and
/// padding; pa, pb, pc and pd go `r3` bytes apart (2 for a packed matrix, 8
/// straight into OAM), and the next entry starts `4 * r3` after.
///
/// This was a stub that wrote 0x100 and 0 to pa and pb and left pc and pd
/// alone - Pokemon Emerald draws every affine sprite through it, so they came
/// out as lines and smears with stack garbage for a matrix.
fn handle_obj_affine_set(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let count = cpu.reg(2).min(AFFINE_SET_MAX);
    let stride = cpu.reg(3);
    for i in 0..count {
        let src_off = src.wrapping_add(i.wrapping_mul(8));
        let sx = i32::from(bus.read16(src_off) as i16);
        let sy = i32::from(bus.read16(src_off.wrapping_add(2)) as i16);
        let m = affine_matrix(sx, sy, bus.read16(src_off.wrapping_add(4)));
        // A negative stride is two's complement, so wrapping u32 maths is the
        // signed offset.
        let dst_off = dst.wrapping_add(stride.wrapping_mul(4).wrapping_mul(i));
        for (k, &v) in m.iter().enumerate() {
            bus.write16(
                dst_off.wrapping_add(stride.wrapping_mul(k as u32)),
                v as u16,
            );
        }
    }
    true
}

/// SWI 0x0D. GBATEK: returns the BIOS checksum, 0xBAAE187F on a GBA. r1 and
/// r3 are what the real routine's loop leaves (mGBA does the same).
fn handle_get_bios_checksum(cpu: &mut Cpu) -> bool {
    cpu.set_reg(0, 0xBAAE_187F);
    cpu.set_reg(1, 1);
    cpu.set_reg(3, 0x4000);
    true
}

/// SWI 0x1F. GBATEK, MidiKey2Freq(r0 = WaveData, r1 = MIDI key, r2 = fine
/// adjust 0-255): `[r0+4] / 2^((180 - key - fine/256) / 12)`. In single
/// precision, as mGBA computes it; the BIOS's own table lookup is not
/// reproduced, so the last digit may differ from hardware.
fn handle_midi_key2freq(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let rate = bus.read32(cpu.reg(0).wrapping_add(4));
    let key = cpu.reg(1) as i32 as f32;
    let fine = cpu.reg(2) as i32 as f32 / 256.0;
    let freq = rate as f32 / ((180.0 - key - fine) / 12.0).exp2();
    cpu.set_reg(0, freq as u32);
    true
}

/// SWI 0x10. GBATEK, BitUnPack(r0 = source, r1 = dest, r2 = info): info is
/// u16 source length in bytes, u8 source unit width (1, 2, 4, 8), u8 dest
/// unit width (1, 2, 4, 8, 16, 32) and u32 offset in bits 0-30 with the
/// zero-data flag in bit 31. Units are taken LSB first from each source byte;
/// the offset is added to every non-zero unit (every unit with the flag), and
/// the results are packed LSB first into words written to the destination.
/// Not masked to the destination width: an overflowing unit ORs into the
/// next, as in mGBA.
///
/// This was a stub that copied bytes until it read a zero.
fn handle_bit_unpack(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let (src, mut dst, info) = (cpu.reg(0), cpu.reg(1), cpu.reg(2));
    let len = u32::from(bus.read16(info));
    let src_width = u32::from(bus.read8(info.wrapping_add(2)));
    let dst_width = u32::from(bus.read8(info.wrapping_add(3)));
    let bias = bus.read32(info.wrapping_add(4));
    if !matches!(src_width, 1 | 2 | 4 | 8) || !matches!(dst_width, 1 | 2 | 4 | 8 | 16 | 32) {
        return true;
    }
    let (offset, zero_too) = (bias & 0x7FFF_FFFF, bias & 0x8000_0000 != 0);
    let mask = (1u32 << src_width) - 1;
    let (mut out, mut filled) = (0u32, 0u32);
    for i in 0..len {
        let byte = u32::from(bus.read8(src.wrapping_add(i)));
        for shift in (0..8).step_by(src_width as usize) {
            let mut unit = (byte >> shift) & mask;
            if unit != 0 || zero_too {
                unit = unit.wrapping_add(offset);
            }
            out |= unit.wrapping_shl(filled);
            filled += dst_width;
            if filled == 32 {
                bus.write32(dst, out);
                dst = dst.wrapping_add(4);
                (out, filled) = (0, 0);
            }
        }
    }
    true
}

/// SWI 0x13. GBATEK, HuffUnComp(r0 = source, r1 = dest): a header (bits 0-3
/// data size 4 or 8, bits 4-7 type 2, bits 8-31 decompressed size), a
/// tree-size byte `n` with the tree in the `(n+1)*2` bytes from it, then the
/// bit stream as 32-bit words read MSB first. A tree node holds the offset to
/// its children in bits 0-5 - they sit at `(addr & !1) + offset*2 + 2` and
/// one byte on - and bits 7 and 6 say child 0 or child 1 is a data leaf.
/// Units are packed LSB first into words, as BitUnPack does.
fn handle_huff_uncomp(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0) & !3;
    let dst = cpu.reg(1);
    let header = bus.read32(src);
    let bits = header & 0xF;
    // 1 would need the tree to output single bits; mGBA declines it too.
    if !matches!(bits, 2 | 4 | 8) {
        return true;
    }
    let size = decompressed_size(header);
    let root = src.wrapping_add(5);
    let mut stream = src.wrapping_add(4 + (u32::from(bus.read8(src.wrapping_add(4))) + 1) * 2);
    // A malformed tree that never reaches a leaf would read forever. A real
    // one needs at most 255 bits per unit, so this bound never cuts one off.
    let max_words = size * 8 + 1;
    let mut out: Vec<u8> = Vec::with_capacity(size + 4);
    let (mut word, mut filled) = (0u32, 0u32);
    let (mut addr, mut node) = (root, bus.read8(root));
    'words: for _ in 0..max_words {
        let data = bus.read32(stream);
        stream = stream.wrapping_add(4);
        for bit in (0..32).rev() {
            let one = (data >> bit) & 1 == 1;
            let child = (addr & !1)
                .wrapping_add(u32::from(node & 0x3F) * 2 + 2)
                .wrapping_add(u32::from(one));
            let leaf = node & if one { 0x40 } else { 0x80 } != 0;
            if !leaf {
                (addr, node) = (child, bus.read8(child));
                continue;
            }
            word |= (u32::from(bus.read8(child)) & ((1 << bits) - 1)) << filled;
            filled += bits;
            (addr, node) = (root, bus.read8(root));
            if filled == 32 {
                out.extend_from_slice(&word.to_le_bytes());
                (word, filled) = (0, 0);
                if out.len() >= size {
                    break 'words;
                }
            }
        }
    }
    write_block(dst, &out, bus);
    true
}

fn handle_lz77_uncomp_wram(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    lz77_decompress(src, dst, bus);
    true
}

fn handle_lz77_uncomp_vram(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    lz77_decompress(src, dst, bus);
    true
}

/// The decompressed byte count from a BIOS decompression header, clamped.
///
/// The field is 24 bits, so a corrupt or hostile ROM can ask for 16 MB - which
/// this would then allocate and spend a long loop filling, on every call. No
/// destination on a GBA is bigger than EWRAM, so anything past 256 KB is a
/// malformed header rather than a request worth honouring.
fn decompressed_size(header: u32) -> usize {
    const MAX: usize = 0x4_0000;
    ((header >> 8) as usize).min(MAX)
}

/// Write a decompressed block out to `dst`.
///
/// This is why the BIOS has separate "Wram" and "Vram" decompressors at all:
/// VRAM, palette RAM and OAM ignore byte stores - a `STRB` there writes the
/// byte into *both* halves of the halfword. Decompressing with byte writes
/// therefore produced garbage in exactly the place the Vram variants exist to
/// serve: every halfword came out as two copies of its second byte, which is
/// why the 240p Test Suite's LZ77-compressed tiles arrived as nothing at all.
fn write_block(dst: u32, data: &[u8], bus: &mut MemoryBus) {
    let halfword_only = (0x0500_0000..0x0800_0000).contains(&dst);
    if !halfword_only {
        for (i, &b) in data.iter().enumerate() {
            bus.write8(dst.wrapping_add(i as u32), b);
        }
        return;
    }
    let mut i = 0;
    while i + 1 < data.len() {
        bus.write16(
            dst.wrapping_add(i as u32),
            u16::from_le_bytes([data[i], data[i + 1]]),
        );
        i += 2;
    }
    if i < data.len() {
        bus.write16(dst.wrapping_add(i as u32), data[i] as u16);
    }
}

/// GBATEK, BIOS Decompression Functions: the header holds the reserved nibble
/// in bits 0-3, the type in bits 4-7 and the **decompressed size in bits 8-31**.
/// The size is the only thing that ends the stream - there is no terminator -
/// so an unbounded loop here runs until it has overwritten every mapped byte.
///
/// The output is built in a buffer rather than written as it is produced. The
/// back-references read from that buffer, so they cannot be corrupted by a
/// destination that mangles the writes, and the finished block goes out
/// through [`write_block`] in units the destination accepts.
fn lz77_decompress(src: u32, dst: u32, bus: &mut MemoryBus) {
    let size = decompressed_size(bus.read32(src));
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut src_pos = src.wrapping_add(4);
    while out.len() < size {
        let flags = bus.read8(src_pos);
        src_pos = src_pos.wrapping_add(1);
        for bit in 0..8 {
            if out.len() >= size {
                break;
            }
            if flags & (0x80 >> bit) != 0 {
                let byte1 = bus.read8(src_pos);
                let byte2 = bus.read8(src_pos.wrapping_add(1));
                src_pos = src_pos.wrapping_add(2);
                let length = ((byte1 >> 4) & 0x0F) as usize + 3;
                // Disp is the full 12 bits, and the +1 applies to the whole
                // displacement: `x | y + 1` would bind the +1 to y alone.
                let offset = ((((byte1 & 0x0F) as usize) << 8) | byte2 as usize) + 1;
                for _ in 0..length {
                    if out.len() >= size {
                        break;
                    }
                    let Some(&val) = out.len().checked_sub(offset).and_then(|i| out.get(i)) else {
                        // A displacement reaching before the start of the
                        // block is a corrupt stream; stop rather than loop.
                        return;
                    };
                    out.push(val);
                }
            } else {
                out.push(bus.read8(src_pos));
                src_pos = src_pos.wrapping_add(1);
            }
        }
    }
    write_block(dst, &out, bus);
}

fn handle_rl_uncomp_wram(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    rl_decompress(src, dst, bus);
    true
}

fn handle_rl_uncomp_vram(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    rl_decompress(src, dst, bus);
    true
}

/// GBATEK, BIOS Decompression Functions: RLUnComp is byte-oriented on both the
/// read and the write side - "Data Byte(s) - N uncompressed bytes, or 1 byte
/// repeated N times". The header carries no unit-size flag; bits 8-31 are the
/// decompressed size, which is what ends the stream.
fn rl_decompress(src: u32, dst: u32, bus: &mut MemoryBus) {
    let size = decompressed_size(bus.read32(src));
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut src_pos = src.wrapping_add(4);
    while out.len() < size {
        let flag = bus.read8(src_pos);
        src_pos = src_pos.wrapping_add(1);
        if flag & 0x80 != 0 {
            // Compressed run: one byte repeated N+3 times.
            let val = bus.read8(src_pos);
            src_pos = src_pos.wrapping_add(1);
            for _ in 0..(flag & 0x7F) as usize + 3 {
                if out.len() >= size {
                    break;
                }
                out.push(val);
            }
        } else {
            // Uncompressed run of N+1 bytes.
            for _ in 0..(flag & 0x7F) as usize + 1 {
                if out.len() >= size {
                    break;
                }
                out.push(bus.read8(src_pos));
                src_pos = src_pos.wrapping_add(1);
            }
        }
    }
    write_block(dst, &out, bus);
}

/// SWI 0x16: Diff8bitUnFilterWrite8bit ("Wram").
///
/// GBATEK, BIOS Decompression Functions: r0 = source (header word with the data
/// size in bits 0-3, the type in bits 4-7 and the decompressed byte count in
/// bits 8-31, followed by the units), r1 = destination. The first unit is the
/// original datum and every later one is the difference from its predecessor,
/// so a running sum reproduces the stream.
fn handle_diff8bit_unfilter_wram(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    // Clamped like the LZ77 and RL headers: the field is 24 bits, so a
    // garbage header asked for up to 16,777,215 read+write pairs - seconds of
    // frozen emulation, with `src + 4 + i` and `dst + i` running off the end
    // of the memory map on the way.
    let size = decompressed_size(bus.read32(src)) as u32;
    let mut sum = 0u8;
    for i in 0..size {
        sum = sum.wrapping_add(bus.read8(src.wrapping_add(4).wrapping_add(i)));
        bus.write8(dst.wrapping_add(i), sum);
    }
    true
}

/// SWI 0x17: Diff8bitUnFilterWrite16bit ("Vram").
///
/// The filter is the 8bit one; only the writes differ, since VRAM rejects byte
/// stores. Units are accumulated in pairs and written as halfwords. An odd
/// trailing byte still costs a halfword write, with the high byte zero.
fn handle_diff8bit_unfilter_vram(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    // Clamped like the LZ77 and RL headers: the field is 24 bits, so a
    // garbage header asked for up to 16,777,215 read+write pairs - seconds of
    // frozen emulation, with `src + 4 + i` and `dst + i` running off the end
    // of the memory map on the way.
    let size = decompressed_size(bus.read32(src)) as u32;
    let mut sum = 0u8;
    let mut pending = 0u16;
    for i in 0..size {
        sum = sum.wrapping_add(bus.read8(src.wrapping_add(4).wrapping_add(i)));
        if i % 2 == 0 {
            pending = sum as u16;
        } else {
            bus.write16(dst.wrapping_add(i - 1), pending | ((sum as u16) << 8));
        }
    }
    if size % 2 == 1 {
        bus.write16(dst.wrapping_add(size - 1), pending);
    }
    true
}

/// SWI 0x18: Diff16bitUnFilter. Same filter over halfword units; the header
/// count stays in bytes, so it covers `size / 2` units.
fn handle_diff16bit_unfilter(cpu: &mut Cpu, bus: &mut MemoryBus) -> bool {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    // Clamped like the LZ77 and RL headers: the field is 24 bits, so a
    // garbage header asked for up to 16,777,215 read+write pairs - seconds of
    // frozen emulation, with `src + 4 + i` and `dst + i` running off the end
    // of the memory map on the way.
    let size = decompressed_size(bus.read32(src)) as u32;
    let mut sum = 0u16;
    for i in 0..(size / 2) {
        sum = sum.wrapping_add(bus.read16(src.wrapping_add(4).wrapping_add(i * 2)));
        bus.write16(dst.wrapping_add(i * 2), sum);
    }
    true
}

fn handle_sound_bias(_cpu: &mut Cpu, _bus: &mut MemoryBus) -> bool {
    true
}
