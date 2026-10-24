package com.gbee.emulator.data

import androidx.annotation.StringRes
import com.gbee.emulator.R

/**
 * How the 240x160 frame is post-processed before it reaches the screen.
 *
 * Two of these cost real per-pixel work every frame, so the trade is stated
 * for each rather than left to be discovered on a slow device.
 */
enum class ScreenFilter {
    /**
     * No processing. The bitmap is drawn with nearest-neighbour sampling, so a
     * scaled-up frame keeps hard pixel edges. Cheapest, and the default.
     */
    NONE,

    /**
     * Bilinear sampling at draw time. Free - it is a flag on `drawImage`, not
     * a pass over the pixels - but it blurs pixel art.
     */
    SMOOTH,

    /**
     * A CRT look: darkened alternate rows drawn over the game.
     *
     * The scanlines are a separate bitmap the GPU tiles over the frame, so the
     * per-frame cost is one extra draw rather than a pass over the pixels.
     */
    SCANLINES,

    /**
     * 2xSaI, doubling 240x160 to 480x320 with edge-aware interpolation.
     *
     * This is the expensive one: 153,600 output pixels of CPU work per frame.
     * It runs on the Compose UI thread inside the composition, so on a slow
     * device it will show up as dropped UI frames rather than as emulation
     * slowdown - the emulation loop is on its own dispatcher and paced by
     * audio.
     */
    SAI_2X,
    ;

    @get:StringRes val label: Int
        get() = when (this) {
            NONE -> R.string.filter_none
            SMOOTH -> R.string.filter_smooth
            SCANLINES -> R.string.filter_scanlines
            SAI_2X -> R.string.filter_sai
        }
}
