# Memory wait states, prefetch and DMA stalls - design

**Status: implemented in PR #69 (`fix/wait-states`, merged 2026-10-08), refined
in #71.** mGBA suite Timing went from 236 to 1880/2020 (total 5208 -> 6852/6998)
with no other sub-suite moving. This file was the proposal; the plan parts are
cut to what was decided, and the full proposal is in git history
(`git show 14a9738:docs/research/wait-states-design.md`). How the model works
today, with the rules fitted from the suite's table, is in `.claude/memory.md`
(entries of 2026-10-08).

Licence rule (see `.claude/memory.md`, Hardware references): mGBA (MPL-2.0)
and NanoBoyAdvance (GPL-3.0) are behaviour references only. Everything here
is derived from GBATEK and from the suite's own hardware table
(`src/timing.c`, MIT). No code was ported from either emulator.

## What was decided

- **Option B was built**: per-access N/S wait states from WAITCNT
  (`memory/timing.rs`, a 256-entry table rebuilt by `set_waitcnt` on store,
  reset and save-state load), the GamePak prefetch buffer
  (`cpu/prefetch.rs`, WAITCNT bit 14), and the DMA stall for immediate,
  HBlank, VBlank and sound DMA. It landed in the planned order: a neutral
  refactor proven by hashing every step's (PC, cycles) against the base
  build, then CPU internal cycles (multiply `m`, register-shift I, THUMB
  hi-register `nop` costing 3), then region waits, then prefetch, then the
  stall.
- **Rejected**: Option 0 (scale today's constants per region) - the table is
  exact to one cycle, so it gets almost nothing.
- **Partly taken from A+** (sub-instruction I/O timing): a load samples I/O
  after its opcode fetch, so `TMxCNT_L` is published ahead by the next
  fetch's cost, and a DMA has a start latency of 2. The timer was **not**
  moved into the bus.
- **Declined, with reasons**:
  - The 140 BIOS Div/Sqrt/ArcTan/CpuSet cells. They time the real BIOS body,
    which the HLE BIOS does not run, and no BIOS image may enter the repo.
    Fitting per-input costs would be gaming the test.
  - VRAM/OAM/palette contention (+1 while the PPU uses them). No test
    measures it, and it needs per-dot PPU timing (`memory/timing.rs:44`).
  - Option C, a cycle-level scheduler. A rewrite of `Gba::step` that
    re-opens every fitted constant, for perhaps 50-100 cells.
- **What it cost and bought**: games now get hardware CPU speed. Mario Tennis
  went from 142.5k to 84k steps a frame (-20% host cost); Yggdra +10% host
  cost. On the Mi 10T Pro, Yggdra, Mario Tennis, Celeste and 240p hold 60 fps.
- **Still open** (in `ROADMAP.md`, Known accuracy gaps): instructions are
  atomic, so a DMA lands at the end of the instruction it started in, and a
  RAM-only DMA does not let the prefetch buffer fill during the stall.

## What the hardware does

Sources: GBATEK "GBA Memory Map", "GBA System Control" (WAITCNT),
"GBA GamePak Prefetch", "ARM CPU Instruction Cycle Times", "GBA DMA
Transfers". Local copies in `temp/gbatek/`.

### Region timings (cycles for 8/16/32-bit)

| Region | Bus | Cycles |
|---|---|---|
| BIOS, IWRAM, I/O, OAM | 32 | 1/1/1 |
| EWRAM | 16 | 3/3/6 |
| Palette, VRAM | 16 | 1/1/2 |
| GamePak ROM WS0/1/2 | 16 | N or S per WAITCNT; 32-bit = N16 + S16 (or 2 x S16) |
| GamePak SRAM | 8 | 1 + SRAM wait |

Palette/VRAM/OAM gain +1 when the PPU is using them at the same time.
Declined: no test here measures it and modelling it needs
per-dot PPU timing.

### WAITCNT (0x04000204)

Bits 0-1 SRAM (4,3,2,8 waits); 2-3 WS0 N (4,3,2,8); 4 WS0 S (2,1); 5-6 WS1 N;
7 WS1 S (4,1); 8-9 WS2 N; 10 WS2 S (8,1); 14 prefetch enable; 15 read-only
cart type. Access time is 1 + waits. Default 0x0000.

Two rules that are easy to miss:

- **A 32-bit gamepak access is two 16-bit accesses, the second always S.**
- **Sequential access is forced to N at every 128 KiB boundary of ROM.**

### N, S, I per instruction

GBATEK's table: ALU 1S (+1I shift by register, +1S+1N to PC), LDR
1S+1N+1I, STR 2N, LDM nS+1N+1I, STM (n-1)S+2N, SWP 1S+2N+1I, B/BL 2S+1N,
THUMB BL 3S+1N, SWI/exception 2S+1N, MUL 1S+mI, MLA 1S+(m+1)I, MULL
1S+(m+1)I, MLAL 1S+(m+2)I. `m` is 1..4 from the multiplier's top bytes
(all-zero or all-one for signed; all-zero only for UMULL/UMLAL). Code cycles
take the code region's timing, the 1N of a data access (and the (n-1)S of
LDM/STM) take the data region's.

**Prefetch Disable Bug** (GBATEK): with prefetch off, a ROM opcode with
internal cycles that does not write R15 turns its following opcode fetch
from S into N.

### Prefetch buffer

Up to eight halfwords, opcode fetches from ROM only. It fills while the CPU
is not using the gamepak bus (internal cycles, accesses to other regions),
and a fetch that hits it costs 1 cycle. A branch, or any data access to the
gamepak, ends the sequential run.

### DMA

`2N + 2(n-1)S + xI`, x = 2, or 4 when both ends are in the gamepak. The CPU is
stopped during the transfer. After an enable edge "wait 2 clock cycles before
accessing any DMA related registers" - there is a start latency.

### Verified convention (hand-checked against the suite's table)

A per-instruction model reproduces the table's prefetch-off columns exactly
with this one convention:

> Every instruction pays one code fetch (pc + 2L) as S, **or N if the previous
> cycle was a data access, a refill, or (ROM, prefetch off) an internal
> cycle**. A write to PC adds a refill N + S at the target, in the target's
> region.

The suite measures `value(test) - value(calibration)`. Both share the same
START (`str` to TM0CNT) and END (`ldrh` from TM0CNT_L), so the measured value
is `cost(CODE) + fetch(END after CODE) - fetch(END after START)`, and START
always leaves END's fetch N. Worked cells (WAITCNT 0x0000: ROM N16 = 5,
S16 = 3, so ARM N32 = 8, S32 = 6):

| Test | Column | Derivation | Table |
|---|---|---|---|
| nop | ARM/ROM ... | N8 + (S6 - N8) | 6 |
| ldr r2, [sp] | ARM/ROM ... | N8 + 1 + I1 + (N8 - N8) | 10 |
| ldr r2, [sp] | ARM/ROM ..S (S16 = 2) | N7 + 1 + 1 | 9 |
| ldrh r2, [0x08000000] | ARM/ROM ... | N8 + N5 + 1 | 14 |
| ldr r2, [0x08000000] | ARM/ROM ... | N8 + (N5 + S3) + 1 | 17 |
| ldr r2, [0x08000000] | ARM/IWRAM | 1 + 8 + 1 | 10 |
| mul #0, #0xFF | ARM/ROM ... | N8 + I1 + disable-bug N | 9 |
| b 1f; nop; 1: nop | ARM/ROM ... | N8 + (N8 + S6) + S6 + (S6 - N8) | 26 |
| Trivial loop | ARM/ROM ... | 8 + 6 + 16x12 + 15x20 + 6 - 2 | 510 |
| Trivial loop | ARM/WRAM | 12 + 192 + 270 + 6 + 0 | 480 |
| mul #X | ARM/IWRAM | 1 + m(X) | 2,2,3,4,5,4,3,2,2,2 |
| umull #X | ARM/IWRAM | 1 + m(X) + 1, unsigned m | 3,3,4,5,6,6,6,6,6,6 |
| Trivial DMA (16) | ARM/ROM ... | str 9 + DMA (2N + 2I = 4) | 13 |

The prefetch-on columns cannot be derived by hand this way and need the
buffer model.
