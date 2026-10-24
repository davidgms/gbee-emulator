package com.gbee.emulator.data

import androidx.annotation.PluralsRes
import com.gbee.emulator.R
import java.util.concurrent.TimeUnit

/**
 * "6 days ago" rather than "3 Sep, 21:41".
 *
 * A date has to be read by doing arithmetic against today; an age is read at a
 * glance, which is the only thing this line is for. The exact timestamp is
 * still one long press away in the info dialog.
 */
object RelativeTime {

    /**
     * An age as a resource and a count, so the wording (and its plural forms)
     * lives in strings.xml. [plural] is 0 for "just now", which has no count.
     */
    data class Age(@PluralsRes val plural: Int, val count: Int = 0) {
        companion object {
            val JUST_NOW = Age(0)
        }
    }

    /**
     * How long ago [thenMillis] was, relative to [nowMillis].
     *
     * Deliberately coarse: this is one subtitle in a list, and
     * `DateUtils.getRelativeTimeSpanString` would drag in a `Context` for a
     * value a test cannot then pin.
     */
    fun ago(thenMillis: Long, nowMillis: Long = System.currentTimeMillis()): Age {
        val delta = nowMillis - thenMillis
        if (delta < 0) return Age.JUST_NOW // a clock that moved backwards, not the future
        val minutes = TimeUnit.MILLISECONDS.toMinutes(delta)
        if (minutes < 1) return Age.JUST_NOW
        if (minutes < 60) return Age(R.plurals.ago_minutes, minutes.toInt())
        val hours = TimeUnit.MILLISECONDS.toHours(delta)
        if (hours < 24) return Age(R.plurals.ago_hours, hours.toInt())
        val days = TimeUnit.MILLISECONDS.toDays(delta)
        if (days < 7) return Age(R.plurals.ago_days, days.toInt())
        if (days < 30) return Age(R.plurals.ago_weeks, (days / 7).toInt())
        // Capped at 11: a 30-day month runs out before a 365-day year does,
        // so days 360 to 364 would otherwise read "12 months ago" - a phrase
        // that means a year while insisting it is not one.
        if (days < 365) return Age(R.plurals.ago_months, (days / 30).coerceAtMost(11).toInt())
        return Age(R.plurals.ago_years, (days / 365).toInt())
    }
}
