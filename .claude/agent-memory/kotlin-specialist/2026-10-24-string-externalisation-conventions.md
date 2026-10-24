---
name: string-externalisation-conventions
description: How hardcoded UI text was moved to strings.xml (enums, VM, plurals) and the compose pitfalls hit
metadata:
  type: project
---

- Enums with UI labels carry `@StringRes val label: Int` (data/ControlTint.kt, ScreenFilter.kt, RomSortOrder...); callers do `stringResource(x.label)`.
- `EmulationViewModel` is an AndroidViewModel: `text(@StringRes, vararg)` helper at the end of the file resolves via the Application context. Slot messages are 2 whole sentences each (quick/numbered) chosen by `slotMessage`.
- `RelativeTime.ago` returns a language-neutral `Age(plural id, count)`; UI resolves it with `relativeAgo` in `ui/Resources.kt`.
- `pluralStringResource` is experimental in the pinned Compose BOM (2024.06); `quantityString` in `ui/Resources.kt` wraps `resources.getQuantityString` instead.
- `joinToString { stringResource(..) }` fails to compile (non-inline lambda); resolve a name map first (see `keyLabels()` in CustomButtonDialog.kt).
- `lintDebug` already fails on 10 pre-existing errors (NewApi in Haptics/PerformanceHint/SaveImport, produceState x3, StateFlow.value); not caused by strings work.
