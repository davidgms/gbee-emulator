package com.gbee.emulator.data

import androidx.annotation.StringRes
import com.gbee.emulator.R

/**
 * How hard the phone answers a press of an on-screen button.
 *
 * A touch button gives nothing back on its own, which is the largest single
 * difference between playing on a screen and playing on a pad: a thumb has no
 * way to know a press landed except by watching the game for it.
 *
 * Amplitudes rather than one canned effect, because a control that fires many
 * times a second has to be quieter than a notification. `VibrationEffect`
 * takes 1-255; the top step here is deliberately well under full power.
 */
enum class HapticStrength(@StringRes val label: Int, val amplitude: Int, val millis: Long) {
    OFF(R.string.haptic_off, 0, 0),
    LIGHT(R.string.haptic_light, 40, 8),
    MEDIUM(R.string.haptic_medium, 90, 12),
    STRONG(R.string.haptic_strong, 160, 16),
}
