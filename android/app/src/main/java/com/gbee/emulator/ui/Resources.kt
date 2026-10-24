package com.gbee.emulator.ui

import androidx.annotation.PluralsRes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import com.gbee.emulator.R
import com.gbee.emulator.data.INTERNAL_STORAGE
import com.gbee.emulator.data.RelativeTime

/** `pluralStringResource` is still experimental in the Compose version this app pins. */
@Composable
@ReadOnlyComposable
fun quantityString(@PluralsRes id: Int, count: Int, vararg args: Any): String {
    LocalConfiguration.current // re-read when the locale changes
    return LocalContext.current.resources.getQuantityString(id, count, *args)
}

/** "6 days ago", from the language-neutral [RelativeTime.Age]. */
@Composable
fun relativeAgo(age: RelativeTime.Age): String =
    if (age.plural == 0) stringResource(R.string.ago_just_now) else quantityString(age.plural, age.count, age.count)

/**
 * A folder label with its internal-storage root in the player's language.
 * [com.gbee.emulator.data.docIdLabel] stays language-free, so the scan and its
 * tests need no Context.
 */
@Composable
fun storageLabel(label: String): String =
    if (label == INTERNAL_STORAGE || label.startsWith("$INTERNAL_STORAGE/")) {
        stringResource(R.string.internal_storage) + label.removePrefix(INTERNAL_STORAGE)
    } else {
        label
    }
