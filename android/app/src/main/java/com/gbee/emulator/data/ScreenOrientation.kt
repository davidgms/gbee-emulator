package com.gbee.emulator.data

import androidx.annotation.StringRes
import com.gbee.emulator.R

/**
 * Which way up the app is allowed to sit.
 *
 * Three states rather than the boolean this replaces. "Force portrait, or
 * not" could lock upright or let go, and had no way to say *landscape* - which
 * is what someone holding the phone sideways for a whole session actually
 * wants, and what stops the picture flipping away every time they lean back.
 */
enum class ScreenOrientation(@StringRes val label: Int) {
    /** Follows the phone, whatever its own rotation lock says. */
    AUTO(R.string.orientation_auto),
    PORTRAIT(R.string.orientation_portrait),
    LANDSCAPE(R.string.orientation_landscape),
}
