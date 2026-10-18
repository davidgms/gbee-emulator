---
name: testing-jni-only-logic-on-host
description: How to give a red/green test to logic inside core/src/ffi.rs `mod android` (cfg target_os=android), which host cargo test never compiles
metadata:
  type: project
---

`core/src/ffi.rs` `pub mod android` is `#[cfg(target_os = "android")]`, so a
host `cargo test` cannot run anything in it. Pattern used for P5-RE-4/5
(2026-10-17): pull the decision into a host-compiled helper next to `guarded`
(`jni_len`, `guarded_status`, `MAX_PEEK_BYTES`, `MAX_AUDIO_SAMPLES`), mark
android-only items `#[cfg_attr(not(target_os = "android"), allow(dead_code))]`,
and test it in `mod boundary_tests` at the END of ffi.rs (clippy
`items_after_test_module` fires otherwise).

Red stage: write the helper first with the OLD behaviour, run the test (fails),
then fix. That proves the test without a JNI harness.

Kotlin audio buffer that bounds nativeAudioCopy: AudioOutput.MAX_SAMPLES_PER_FRAME
(810) x (DisplaySettings.MAX_FAST_FORWARD_RATIO 8 + 2) = 8100 floats.
