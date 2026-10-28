---
name: check-hle-stubs-before-the-renderer
description: When sprites look wrong, dump the affine matrices from OAM and check the HLE BIOS SWIs the game calls before touching the PPU
metadata:
  type: feedback
---

"Bugged sprites" in Pokemon Emerald were not a PPU bug: OAM dump showed pc/pd garbage
(pc 25808) while pa/pb were exactly 0x100/0 - the fingerprint of the ObjAffineSet stub.

**Why:** the HLE BIOS in `core/src/bios.rs` has stubs that look finished (signatures, caps,
wrapping maths) but compute nothing; a game that calls them every frame renders "wrong".

**How to apply:**
- First probe for a graphics bug: dump OAM attrs + the 32 affine params
  (`oam[k*8+6]` for k in group*4..group*4+4) - `temp/emerald/probe/src/main.rs` does it.
- Then count the SWIs the game issues (`temp/emerald/probe/src/bin/swis.rs`: peek the opcode
  at `cpu.registers[15]` before each `Gba::step`) and audit those handlers.
- After fixing, run 3+ frames: the state's OAM was computed by the old code.
- Audio: the mGBA SDL disk-driver comparison in `.claude/memory.md` (2026-10-27) is objective;
  use it instead of guessing.
