---
name: emulator-bug-research-fetch-craft
description: Which URLs work for pulling emulator issues/changelogs with WebFetch (NBA is on Codeberg, issue-search pages list titles)
metadata:
  type: reference
---

- NanoBoyAdvance issues: `github.com/nba-emu/NanoBoyAdvance/issues/N` is 404; use `codeberg.org/nba-emu/NanoBoyAdvance/issues/N` (flaky "socket hang up", retry). Release notes: `github.com/nba-emu/NanoBoyAdvance/releases`. Source: raw.githubusercontent.com master `src/nba/src/hw/apu/{apu.cc,channel/*.cc,hle/mp2k.cc}` (note `.cc` not `.cpp`).
- `github.com/<repo>/issues?q=is%3Aissue+audio` fetches fine and lists titles: fast way to find audio issues in SkyEmu, gpSP, VBA-M, mGBA.
- mGBA CHANGES is 112k chars; WebFetch covers 100k, use `offset: 100000` for the rest. mGBA `mgba.io/i/N` redirects; use the github issue URL.
- Search snippets are not citations; only fetched pages count. Meteor and gpSP audio issues are nearly nonexistent online.
- Result of the 2026-10-27 survey: temp/bug-research/B-emulator-sound-bugs.md (gitignored, may be gone).
