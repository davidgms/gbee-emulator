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
 * [roms] without the [hidden] copies - but only while a copy of the same
 * game is still listed. If the kept file is deleted or its folder removed,
 * the hidden one comes back rather than the game vanishing with no row.
 */
internal fun withoutHidden(roms: List<RomEntry>, hidden: Set<String>): List<RomEntry> {
    val shown = roms.filter { it.exists && it.filePath !in hidden }.mapTo(HashSet(), ::duplicateKey)
    return roms.filterNot { it.filePath in hidden && duplicateKey(it) in shown }
}

/**
 * The player's answers to "which copy do you keep?". [hidden] are the copies
 * left out of the list; [settled] are groups where both were kept. Both
 * persist, so a rescan does not ask about the same files twice.
 */
class DuplicateChoices(private val context: Context) {
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
            // The game's star and last-played time follow it onto the copy kept.
            val moved = drop.associateWith { keep.filePath }
            LastPlayed(context).rekey(moved)
            Favorites(context).rekey(moved)
        }
    }

    /** Forget every answer, so the next scan asks about each game found twice again. */
    fun reset() {
        prefs.edit().clear().apply()
    }

    private companion object {
        const val HIDDEN = "hidden_paths"
        const val SETTLED = "kept_groups"
    }
}
