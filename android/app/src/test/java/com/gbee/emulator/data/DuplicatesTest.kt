package com.gbee.emulator.data

import org.junit.Assert.assertEquals
import org.junit.Test

class DuplicatesTest {
    private fun rom(file: String, path: String = "/roms/$file", exists: Boolean = true) =
        RomEntry(
            name = file.substringBeforeLast('.').replace("_", " "),
            fileName = file,
            size = "1 MB",
            filePath = path,
            exists = exists,
        )

    @Test
    fun `a zip and a gba of one game are one group`() {
        val zip = rom("Mario Tennis.zip")
        val gba = rom("mario_tennis.gba")
        val other = rom("Metroid.gba")
        assertEquals(listOf(listOf(zip, gba)), duplicateGroups(listOf(zip, other, gba), emptySet()))
    }

    @Test
    fun `a settled group is not asked again until it gains a copy`() {
        val a = rom("Mario Tennis.zip")
        val b = rom("Mario Tennis.gba")
        val settled = setOf(groupId(listOf(b, a)))
        assertEquals(emptyList<List<RomEntry>>(), duplicateGroups(listOf(a, b), settled))
        val c = rom("Mario Tennis.gba", path = "/sd/Mario Tennis.gba")
        assertEquals(1, duplicateGroups(listOf(a, b, c), settled).size)
    }

    @Test
    fun `a missing row is not a copy`() {
        val gone = rom("Mario Tennis.gba", exists = false)
        assertEquals(emptyList<List<RomEntry>>(), duplicateGroups(listOf(rom("Mario Tennis.zip"), gone), emptySet()))
    }

    @Test
    fun `a hidden copy stays hidden only while another copy is listed`() {
        val zip = rom("Mario Tennis.zip")
        val gba = rom("Mario Tennis.gba")
        val hidden = setOf(zip.filePath)
        assertEquals(listOf(gba), withoutHidden(listOf(zip, gba), hidden))
        // The kept .gba was deleted: the .zip comes back instead of the game vanishing.
        assertEquals(listOf(zip), withoutHidden(listOf(zip), hidden))
    }
}
