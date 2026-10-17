package com.gbee.emulator.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PhaseFiveDownloadsTest {

    @Test
    fun `badge bounds accept a normal badge and refuse a decompression bomb`() {
        assertTrue(BadgeCache.isSaneBadgeSize(64, 64))
        assertTrue(BadgeCache.isSaneBadgeSize(512, 512))
        assertFalse(BadgeCache.isSaneBadgeSize(513, 64))
        assertFalse(BadgeCache.isSaneBadgeSize(30000, 30000))
        assertFalse(BadgeCache.isSaneBadgeSize(-1, -1))
        assertFalse(BadgeCache.isSaneBadgeSize(0, 64))
    }

    @Test
    fun `user agent carries version and release only`() {
        val ua = httpUserAgent("1.2.3", "14")
        assertEquals("GBee/v1.2.3 (Android 14)", ua)
        assertFalse(ua.contains("Build/"))
    }

    @Test
    fun `cover base name cannot escape the covers folder`() {
        assertEquals("", CoverArt.safeBaseName("../x"))
        assertEquals("", CoverArt.safeBaseName(".."))
        assertEquals("", CoverArt.safeBaseName("a/b"))
        assertEquals("", CoverArt.safeBaseName("a\\b"))
        assertEquals("Pokemon", CoverArt.safeBaseName("Pokemon"))
        assertEquals("Pokemon - Emerald (USA)", CoverArt.safeBaseName("Pokemon - Emerald (USA)"))
    }
}
