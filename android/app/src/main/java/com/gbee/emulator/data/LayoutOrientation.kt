package com.gbee.emulator.data

import androidx.annotation.StringRes
import com.gbee.emulator.R

/**
 * Which way the phone has to be held for a layout to be offered.
 *
 * A portrait arrangement does not survive the turn: the picture moves, the
 * controls flank it instead of floating under it, and a custom button's
 * absolute position lands somewhere else entirely. Rather than guess a
 * translation, a layout says where it belongs and the other orientation gets
 * a layout of its own.
 */
enum class LayoutOrientation(@StringRes val label: Int) {
    PORTRAIT(R.string.orientation_portrait),
    LANDSCAPE(R.string.orientation_landscape),
    BOTH(R.string.orientation_both);

    fun appliesTo(landscape: Boolean): Boolean = when (this) {
        PORTRAIT -> !landscape
        LANDSCAPE -> landscape
        BOTH -> true
    }

    companion object {
        /** What a layout saved before this setting existed is taken to be. */
        val DEFAULT = BOTH

        fun fromName(name: String?): LayoutOrientation =
            entries.firstOrNull { it.name == name } ?: DEFAULT
    }
}
