# Save data: battery saves and save states

How the two kinds of save work, why the boundary sits where it does, and what
is still missing. Written 2026-08-28, after wiring the cartridge into the bus.

## The two kinds, and why they are not the same thing

- **Battery save** - what the game itself writes when you save in-game. It is
  the cartridge's SRAM, Flash or EEPROM chip. The game owns the format; we only
  store bytes. Losing one loses the player's actual progress.
- **Save state** - a snapshot of the whole emulator. Ours, not the game's, and
  tied to our internal layout, so it breaks whenever that layout changes.

They fail differently and should be stored differently. A corrupted save state
costs one slot. A corrupted battery save costs the playthrough.

## What was actually wrong

`Cartridge::save_read`, `save_write`, `save_data` and `load_save` existed and
were complete, and **had zero callers**. `MemoryBus` had no `Cartridge` field
and no arm for `0x0E000000`, so every in-game save write was silently
discarded and every read returned 0. The Flash command state machine had never
executed a single transition.

This was not a persistence gap. The save hardware was unreachable.

## The fix, and the design decisions in it

### The cartridge lives in the bus

`MemoryBus` owns the `Cartridge`. It is the only object reachable from
`read8`/`store8`, which see a `&mut MemoryBus` and nothing else, so anything
the memory path must reach has to live there. `Gba` keeps thin accessors
(`save_data`, `load_save`, `take_save_dirty`, `cartridge`).

### The save region has an 8-bit databus

GBATEK, *GBA Cart Backup SRAM/FRAM*: the region is at `0x0E000000` and "the
databus is restricted to 8 bits, it should be accessed by LDRB, LDRSB, and
STRB opcodes only".

That is not a footnote, it changes observable behaviour:

- A **halfword read** returns the single byte replicated. `STRB 1` then `LDRH`
  gives `0x0101`, not `0xFF01`.
- A **halfword write** delivers only the byte selected by the accessed
  address, so `STRH 0xAABB` writes `0xBB` at an even address and `0xAA` at an
  odd one.

Because of that, **access alignment is forced inside `MemoryBus`, not by the
callers**. The CPU used to mask the address before calling `write16`, which
threw away the low bit the save region needs. Any new caller should pass the
raw address and let the bus decide.

### Flash erase needs two unlock sequences

The full chip-erase command is `AA, 55, 80, AA, 55, 10`. The state machine
originally took the command straight after `0x80`, which swallowed the second
`0xAA` and meant the erase never ran - a game that formats its save before
writing would find the old contents still there. Sector erase (`0x30`) has the
same shape.

### The dirty flag is set at the mutation sites

`Cartridge::save_dirty` is set where a byte actually changes - the SRAM store,
the Flash byte-program, and the erase paths - not at the top of `save_write`.
The Flash unlock bytes `0xAA`/`0x55` change no data, and treating them as
dirty would trigger a file write every time a game so much as touches Flash.

**Ordering contract**: check `take_save_dirty()` **then** read the bytes.
Reversed, a write landing between the read and the clear is lost. Today every
call happens on the emulation thread between frames so there is no race, but
the contract has to hold the moment anyone moves the flush elsewhere.

## The FFI surface

Four functions, plus JNI mirrors. Deliberately small.

| Function | Purpose |
|---|---|
| `gbee_save_size` | Bytes in the battery save, 0 if the cart has no save chip |
| `gbee_save_read` | Copy the save out into a caller-owned buffer |
| `gbee_save_write` | Restore a save |
| `gbee_save_take_dirty` | Did save memory change since last asked; clears |

There is deliberately **no `save_type` function**. The size uniquely identifies
the type (512 / 32K / 64K / 8K / 128K are all distinct) and the frontend's real
questions are "how many bytes" and "is there a save at all". Add one later if a
UI genuinely wants to print "Flash 128K".

Save states use the same shape, so the frontend has one pattern rather than
two:

| Function | Purpose |
|---|---|
| `gbee_state_size` | Bytes a state taken now would occupy |
| `gbee_state_read` | Copy a state out; 0 if the buffer is too small |
| `gbee_state_write` | Restore a state; -1 if rejected |

This **replaced** an opaque-handle API (`save_state_create` / `load_state` /
`save_state_destroy`) that handed Kotlin a `Long` it could only give back or
free. States were not exportable to a file at all under that design, which is
why "save states are not wired to a UI" understated the problem.

A `-1` from `state_write` leaves the emulator **untouched**. `SaveState::restore`
snapshots the machine before parsing and rolls back if the parse fails, so a
bad file costs the load, not the session: report it and carry on. (This was
once the other way round - a rejected state left a hybrid of two machines -
and the frontends still carried "reload the ROM" wording long after the core
stopped needing it.)

`state_write` also returns `-1` for a state that parses but describes a machine
that cannot exist (timer reload past 16 bits, `cycles` past 2^62, an APU sample
accumulator past one period, an EEPROM count past its phase), and, from state
v10, for a state taken on a different ROM: v10 appends the ROM's CRC-32 and a
mismatch is refused, because the state carries the battery save and would
otherwise replace this cart's. v6-v9 states carry no CRC and still load.

v11 appends the live state a machine carries between two instructions: the
PPU's dot within the scanline and whether it has drawn the line, the affine
reference accumulators, the HLE `IntrWait`/`Halt` flags, the BIOS read latch,
and the cycles owed to the next step (exception entry, a DMA stall, the IRQ
delay). Before it a restored machine kept the values of whatever it was
running, so a state saved in `VBlankIntrWait` woke a frame late and the line
timing moved. v6-v10 states still load with those derived or reset; see
`core/tests/state_resume.rs`. Out-of-range values are refused like the rest.

There is deliberately **no path-taking FFI**. Android hands out content URIs
and file descriptors, not paths the core can `fs::write`. Storage policy is the
frontend's job; bytes in, bytes out is the only honest boundary.

The core also does **not** auto-flush inside `run_frame`. It has no clock, no
filesystem and no lifecycle. `take_save_dirty` returning true is information;
deciding whether that means "write now", "write in two seconds" or "write on
`onPause`" belongs to the frontend.

## Verified by

`core/tests/saves.rs`, `core/tests/state_restore.rs` and
`core/tests/state_resume.rs`, plus these ROMs from `jsmolka/gba-tests` driven
through `core/tests/gba_suite.rs`:

| ROM | Status |
|---|---|
| `sram.gba` | passes |
| `flash64.gba` | passes |
| `flash128.gba` | passes |
| `none.gba` | passes |
| `bios.gba` | passes since the bus answers BIOS reads from `MemoryBus::bios_latch` (#67) |

ROMs are never committed. Fetch them into `temp/roms/`. The ROM tests print
"skipping" and **pass** when the file is absent, so on a fresh clone or in CI
they are not a gate; `saves.rs` is.

## Save state format history

The current format is **v11** (`SAVE_VERSION` in `core/src/savestate.rs`);
v6 to v11 load, anything older is refused. Milestones: v3 made the round trip
real (timers came back disabled, DMA-derived state stale, `io_regs` and the
cart save absent, and a rejected restore left a hybrid of two machines - it
now snapshots and rolls back); v4 added the APU; v8 the DMA latches (#65);
v9 SIO (#68); v10 the ROM CRC-32; v11 the live between-step state (#94). The
v10 and v11 paragraphs above describe what those two carry.

## Still missing

Done since this file was written, checked in code: Android persistence
(battery saves live only in app storage, `filesDir/saves/<key>.sav`, since
#78, with a one-time import of the old `.sav` beside the ROM - `RomFiles.kt`),
Flash chip ID reads (`flash_reports_its_chip_id`), the EEPROM width taken from
the DMA length and the game's save size rather than guessed
(`the_eeprom_address_width_follows_the_dma_length`,
`an_8kb_eeprom_on_a_small_cart_keeps_every_block`), and APU state in the save
state (v4).

Still open:

- **No RTC.** There is no GPIO or S3511 clock at `0x080000C4`, so Pokemon
  Ruby/Sapphire/Emerald report a dead battery and lose clock events.
- **Flash byte-program overwrites** (`cart/mod.rs`, state 4). Real flash can
  only clear bits, so a game that programs without erasing gets different
  data. No Atmel page-write variant.
- **Save-type detection has no override list.** A ROM with a stripped or
  misleading library string gets `SaveType::None` and its saves are lost.
