package com.gbee.emulator.ui.screens

import com.gbee.emulator.R
import androidx.compose.ui.res.stringResource
import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.gbee.emulator.data.CustomButton
import com.gbee.emulator.data.CustomButtonMode
import com.gbee.emulator.engine.GbaEngine
import com.gbee.emulator.ui.theme.*
import java.util.UUID

/** name (string resource), GBA key, in the order the picker shows them. */
private val PICKABLE_KEYS = listOf(
    R.string.key_up to GbaEngine.KEY_UP,
    R.string.key_down to GbaEngine.KEY_DOWN,
    R.string.key_left to GbaEngine.KEY_LEFT,
    R.string.key_right to GbaEngine.KEY_RIGHT,
    R.string.button_l to GbaEngine.KEY_L,
    R.string.button_r to GbaEngine.KEY_R,
    R.string.button_a to GbaEngine.KEY_A,
    R.string.button_b to GbaEngine.KEY_B,
    R.string.button_start to GbaEngine.KEY_START,
    R.string.button_select to GbaEngine.KEY_SELECT,
)

/** Resolves the key names once, so a `joinToString` lambda does not need to be composable. */
@Composable
private fun keyLabels(): (Int) -> String {
    val names = PICKABLE_KEYS.associate { (name, key) -> key to stringResource(name) }
    return { key -> names[key] ?: "?" }
}

@get:StringRes
private val CustomButtonMode.label: Int
    get() = when (this) {
        CustomButtonMode.COMBO -> R.string.mode_combo
        CustomButtonMode.SEQUENCE -> R.string.mode_sequence
        CustomButtonMode.TOGGLE_HOLD -> R.string.mode_toggle_hold
    }

@get:StringRes
private val CustomButtonMode.description: Int
    get() = when (this) {
        CustomButtonMode.COMBO -> R.string.mode_combo_description
        CustomButtonMode.SEQUENCE -> R.string.mode_sequence_description
        CustomButtonMode.TOGGLE_HOLD -> R.string.mode_toggle_hold_description
    }

/**
 * Lists a layout's custom buttons and manages them - create, edit, delete.
 * Positioning and dragging them happens on the layout canvas itself, the
 * same as the real buttons; this dialog only owns what each one is.
 */
@Composable
fun CustomButtonsListDialog(
    buttons: List<CustomButton>,
    onSave: (CustomButton) -> Unit,
    onDelete: (CustomButton) -> Unit,
    onDismiss: () -> Unit,
) {
    val keyLabel = keyLabels()
    var editing by remember { mutableStateOf<CustomButton?>(null) }
    var creating by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = NightPanel,
        titleContentColor = GoldenSaplight,
        textContentColor = PineGlowMist,
        title = { Text(stringResource(R.string.custom_buttons_title), fontWeight = FontWeight.Bold) },
        text = {
            Column(modifier = Modifier.verticalScroll(rememberScrollState())) {
                if (buttons.isEmpty()) {
                    Text(
                        stringResource(R.string.custom_buttons_empty),
                        color = PineGlowMist.copy(alpha = 0.7f),
                        fontSize = 13.sp,
                    )
                }
                buttons.forEach { button ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(button.name, color = PineGlowMist, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                            Text(
                                stringResource(
                                    R.string.custom_button_summary,
                                    stringResource(button.mode.label),
                                    button.keys.joinToString(" ") { keyLabel(it) },
                                ),
                                color = AmberResin,
                                fontSize = 11.sp,
                            )
                        }
                        IconButton(onClick = { editing = button }) {
                            Icon(Icons.Default.Edit, stringResource(R.string.edit), tint = AmberResin)
                        }
                        IconButton(onClick = { onDelete(button) }) {
                            Icon(Icons.Default.Delete, stringResource(R.string.delete), tint = AmberResin)
                        }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = { creating = true }) { Text(stringResource(R.string.custom_button_new), color = GoldenSaplight) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.close), color = PineGlowMist) }
        },
    )

    editing?.let { button ->
        CustomButtonEditDialog(
            existing = button,
            onSave = { saved -> onSave(saved); editing = null },
            onDismiss = { editing = null },
        )
    }

    if (creating) {
        CustomButtonEditDialog(
            existing = null,
            onSave = { saved -> onSave(saved); creating = false },
            onDismiss = { creating = false },
        )
    }
}

/**
 * The form for one custom button: a name, a behaviour, and which keys it
 * drives. Serves both create ([existing] null) and edit.
 */
@Composable
private fun CustomButtonEditDialog(
    existing: CustomButton?,
    onSave: (CustomButton) -> Unit,
    onDismiss: () -> Unit,
) {
    val keyLabel = keyLabels()
    var name by remember { mutableStateOf(existing?.name ?: "") }
    var mode by remember { mutableStateOf(existing?.mode ?: CustomButtonMode.COMBO) }
    var keys by remember { mutableStateOf(existing?.keys ?: emptyList()) }

    fun tapKey(key: Int) {
        keys = when (mode) {
            // A sequence is order- and repeat-sensitive - tapping always
            // appends. A combo or a hold toggle is just a set of keys.
            CustomButtonMode.SEQUENCE -> keys + key
            else -> if (key in keys) keys - key else keys + key
        }
    }

    val canSave = name.isNotBlank() && keys.isNotEmpty()

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = NightPanel,
        titleContentColor = GoldenSaplight,
        textContentColor = PineGlowMist,
        title = { Text(stringResource(if (existing == null) R.string.custom_button_new_title else R.string.custom_button_edit_title)) },
        text = {
            Column(
                modifier = Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    singleLine = true,
                    label = { Text(stringResource(R.string.custom_button_name)) },
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = PineGlowMist,
                        unfocusedTextColor = PineGlowMist,
                        focusedBorderColor = AmberResin,
                        unfocusedBorderColor = AmberResin.copy(alpha = 0.5f),
                        cursorColor = AmberResin,
                    ),
                )

                Column {
                    Text(stringResource(R.string.custom_button_behaviour), color = AmberResin, fontSize = 12.sp)
                    CustomButtonMode.entries.forEach { m ->
                        // The row and the radio are two separate touch targets
                        // for the same choice, so they share one handler. They
                        // did not: the row cleared `keys` unconditionally while
                        // the radio kept them, so whether editing a button lost
                        // its keys depended on whether the tap landed on the
                        // label or on the radio itself.
                        val selectMode = {
                            if (mode != m) {
                                // Only a move across the SEQUENCE boundary
                                // invalidates the picks: SEQUENCE keeps an
                                // ordered list that may repeat a key, the
                                // other two an unordered set.
                                val crossesSequence =
                                    (mode == CustomButtonMode.SEQUENCE) !=
                                        (m == CustomButtonMode.SEQUENCE)
                                mode = m
                                if (crossesSequence) {
                                    keys = emptyList()
                                }
                            }
                        }
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable(onClick = selectMode),
                        ) {
                            RadioButton(
                                selected = mode == m,
                                onClick = selectMode,
                                colors = RadioButtonDefaults.colors(
                                    selectedColor = GoldenSaplight,
                                    unselectedColor = AmberResin,
                                ),
                            )
                            Column {
                                Text(stringResource(m.label), color = PineGlowMist, fontSize = 13.sp)
                                Text(stringResource(m.description), color = PineGlowMist.copy(alpha = 0.6f), fontSize = 10.sp)
                            }
                        }
                    }
                }

                Column {
                    Text(
                        stringResource(if (mode == CustomButtonMode.SEQUENCE) R.string.keys_in_order else R.string.keys),
                        color = AmberResin,
                        fontSize = 12.sp,
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    KeyPickerGrid(
                        selected = keys.toSet(),
                        highlightSelection = mode != CustomButtonMode.SEQUENCE,
                        onTap = ::tapKey,
                    )
                }

                if (mode == CustomButtonMode.SEQUENCE) {
                    Text(
                        text = if (keys.isEmpty()) {
                            stringResource(R.string.sequence_empty)
                        } else {
                            stringResource(R.string.sequence_keys, keys.joinToString(" → ") { keyLabel(it) })
                        },
                        color = PineGlowMist,
                        fontSize = 12.sp,
                    )
                    Row {
                        TextButton(onClick = { keys = keys.dropLast(1) }, enabled = keys.isNotEmpty()) {
                            Text(stringResource(R.string.undo_last), color = AmberResin, fontSize = 12.sp)
                        }
                        TextButton(onClick = { keys = emptyList() }, enabled = keys.isNotEmpty()) {
                            Text(stringResource(R.string.clear), color = AmberResin, fontSize = 12.sp)
                        }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                enabled = canSave,
                onClick = {
                    onSave(
                        CustomButton(
                            id = existing?.id ?: UUID.randomUUID().toString(),
                            name = name.trim(),
                            mode = mode,
                            keys = keys,
                        )
                    )
                },
            ) { Text(stringResource(R.string.save), color = if (canSave) GoldenSaplight else PineGlowMist.copy(alpha = 0.4f)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel), color = PineGlowMist) }
        },
    )
}

/** The ten real keys as tappable chips, two rows so it works without a
 *  flow-layout dependency. */
@Composable
private fun KeyPickerGrid(
    selected: Set<Int>,
    highlightSelection: Boolean,
    onTap: (Int) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        PICKABLE_KEYS.chunked(5).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                row.forEach { (label, key) ->
                    val isSelected = highlightSelection && key in selected
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(8.dp))
                            .background(if (isSelected) GoldenSaplight else NightVoid)
                            .clickable { onTap(key) }
                            .padding(horizontal = 10.dp, vertical = 8.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            stringResource(label),
                            color = if (isSelected) BurntRoot else PineGlowMist,
                            fontSize = 12.sp,
                            fontWeight = FontWeight.SemiBold,
                        )
                    }
                }
            }
        }
    }
}
