package com.gbee.emulator.data

import android.content.Context

/**
 * The key two copies of one game share: the display name, so
 * `Mario Tennis.zip` and `Mario_Tennis.gba` meet. By name only - the scan
 * reads no file, and opening every zip to compare headers is what it avoids.
 */
internal fun duplicateKey(rom: RomEntry): String = rom.name.trim().lowercase()

/** The ids of one group, in a fixed order, as [DuplicateChoices] stores "keep both". */
internal fun groupId(group: List<RomEntry>): String = group.map { it.filePath }.sorted().joinToString("\n")

/**
 * Groups of two or more files on storage that look like one game, leaving
 * out the groups the player already settled. A group that gains a new copy
 * has a new [groupId], so it is asked again.
 */
internal fun duplicateGroups(roms: List<RomEntry>, settled: Set<String>): List<List<RomEntry>> =
    roms.filter { it.exists }
        .groupBy(::duplicateKey)
        .values
        .filter { it.size > 1 && groupId(it) !in settled }

/**
 * The player's answers to "which copy do you keep?". [hidden] are the copies
 * left out of the list; [settled] are groups where both were kept. Both
 * persist, so a rescan does not ask about the same files twice.
 */
class DuplicateChoices(context: Context) {
    private val prefs = context.getSharedPreferences("duplicates", Context.MODE_PRIVATE)

    fun hidden(): Set<String> = prefs.getStringSet(HIDDEN, emptySet()) ?: emptySet()

    fun settled(): Set<String> = prefs.getStringSet(SETTLED, emptySet()) ?: emptySet()

    /** Keep [keep] and hide the rest of [group]; a null [keep] keeps them all. */
    fun resolve(group: List<RomEntry>, keep: RomEntry?) {
        // New Set instances: SharedPreferences keeps the one it was given.
        if (keep == null) {
            prefs.edit().putStringSet(SETTLED, settled() + groupId(group)).apply()
        } else {
            val drop = group.map { it.filePath } - keep.filePath
            prefs.edit().putStringSet(HIDDEN, hidden() + drop).apply()
        }
    }

    private companion object {
        const val HIDDEN = "hidden_paths"
        const val SETTLED = "kept_groups"
    }
}
