package com.gbee.emulator.data

import androidx.annotation.StringRes
import com.gbee.emulator.R

/**
 * How large the letters on the on-screen buttons are drawn.
 *
 * Three fixed steps rather than a slider. The labels are one to six
 * characters inside a fixed circle or pill, so there is a narrow band where
 * they are legible and still fit - a free multiplier mostly offers ways to
 * get it wrong.
 *
 * A multiplier and not a size in sp, because the labels do not start equal:
 * A and B are 20sp inside a big circle, START and SELECT are 13sp inside a
 * narrow pill, and a single sp value for both would break one of them.
 */
enum class ControlFontSize(@StringRes val label: Int, val scale: Float) {
    SMALL(R.string.font_size_small, 0.85f),
    MEDIUM(R.string.font_size_medium, 1.0f),
    LARGE(R.string.font_size_large, 1.25f),
}
