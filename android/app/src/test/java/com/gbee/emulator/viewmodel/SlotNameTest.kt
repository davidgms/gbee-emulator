package com.gbee.emulator.viewmodel

import com.gbee.emulator.R
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * The save/load messages have to name the slot the player tapped.
 *
 * They used to be built from `slot + 1`, so saving into the row labelled
 * "Slot 2" reported "Saved to slot 3" - the one visible symptom of an
 * off-by-one that ran through every save-state message.
 */
class SlotNameTest {

    @Test
    fun `slot zero is the quick save message`() {
        assertEquals(R.string.state_saved_quick, EmulationViewModel.slotMessage(0, R.string.state_saved_quick, R.string.state_saved_slot))
    }

    @Test
    fun `every other slot is numbered by its own index`() {
        for (slot in 1 until EmulationViewModel.SLOT_COUNT) {
            assertEquals(
                R.string.state_saved_slot,
                EmulationViewModel.slotMessage(slot, R.string.state_saved_quick, R.string.state_saved_slot),
            )
        }
    }
}
