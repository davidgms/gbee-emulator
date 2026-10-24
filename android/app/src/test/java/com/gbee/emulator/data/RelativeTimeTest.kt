package com.gbee.emulator.data

import com.gbee.emulator.R
import org.junit.Assert.assertEquals
import org.junit.Test

/** "6 days ago" in the list, instead of a date to do arithmetic against. */
class RelativeTimeTest {

    private val now = 1_700_000_000_000L
    private fun ago(millis: Long) = RelativeTime.ago(now - millis, now)

    private val minute = 60_000L
    private val hour = 60 * minute
    private val day = 24 * hour

    @Test
    fun `under a minute is just now`() {
        assertEquals(RelativeTime.Age.JUST_NOW, ago(0))
        assertEquals(RelativeTime.Age.JUST_NOW, ago(59_000))
    }

    @Test
    fun `minutes hours and days`() {
        assertEquals(RelativeTime.Age(R.plurals.ago_minutes, 1), ago(minute))
        assertEquals(RelativeTime.Age(R.plurals.ago_minutes, 45), ago(45 * minute))
        assertEquals(RelativeTime.Age(R.plurals.ago_hours, 1), ago(hour))
        assertEquals(RelativeTime.Age(R.plurals.ago_hours, 23), ago(23 * hour))
        assertEquals(RelativeTime.Age(R.plurals.ago_days, 1), ago(day))
        assertEquals(RelativeTime.Age(R.plurals.ago_days, 6), ago(6 * day))
    }

    @Test
    fun `a week in, it counts weeks`() {
        assertEquals(RelativeTime.Age(R.plurals.ago_weeks, 1), ago(7 * day))
        assertEquals(RelativeTime.Age(R.plurals.ago_weeks, 4), ago(29 * day))
    }

    @Test
    fun `a month in, it counts months`() {
        assertEquals(RelativeTime.Age(R.plurals.ago_months, 1), ago(30 * day))
        assertEquals(RelativeTime.Age(R.plurals.ago_months, 11), ago(364 * day))
    }

    @Test
    fun `a year in, it counts years`() {
        assertEquals(RelativeTime.Age(R.plurals.ago_years, 1), ago(365 * day))
        assertEquals(RelativeTime.Age(R.plurals.ago_years, 2), ago(800 * day))
    }

    /**
     * A phone whose clock moved backwards - a manual change, or a timezone
     * rollback - must not print "-3 hours ago".
     */
    @Test
    fun `a timestamp in the future reads as just now`() {
        assertEquals(RelativeTime.Age.JUST_NOW, RelativeTime.ago(now + day, now))
    }
}
