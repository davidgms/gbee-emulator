package com.gbee.emulator.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.gbee.emulator.ui.theme.GoldenSaplight
import com.gbee.emulator.ui.theme.NightPanel
import com.gbee.emulator.ui.theme.PineGlowMist
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * GBee's own toast, for screens outside a game: the same pill as the in-game
 * messages, drawn once over the whole app in `GBeeNavHost`. A flow and not a
 * screen's state, so a message sent by work that outlives its screen (a save
 * import) still shows.
 */
object AppToast {
    private val flow = MutableStateFlow<String?>(null)
    val message: StateFlow<String?> get() = flow

    fun show(text: String) {
        flow.value = text
    }

    fun clear() {
        flow.value = null
    }
}

@Composable
fun StateToast(
    message: String?,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
    durationMs: Long = 2200,
    icon: androidx.compose.ui.graphics.vector.ImageVector = Icons.Default.CheckCircle,
) {
    // Held locally so the pill can finish fading out after the message is
    // already gone from the ViewModel.
    var shown by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(message) {
        if (message != null) {
            shown = message
            delay(durationMs)
            onDismiss()
        }
    }

    AnimatedVisibility(
        visible = message != null,
        enter = fadeIn(tween(150)) + slideInVertically(tween(180)) { -it / 2 },
        exit = fadeOut(tween(220)),
        modifier = modifier.padding(top = 56.dp),
    ) {
        Row(
            modifier = Modifier
                .clip(RoundedCornerShape(50))
                .background(NightPanel)
                .border(1.dp, GoldenSaplight.copy(alpha = 0.45f), RoundedCornerShape(50))
                .padding(horizontal = 16.dp, vertical = 9.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                icon,
                contentDescription = null,
                tint = GoldenSaplight,
                modifier = Modifier.size(16.dp),
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = shown.orEmpty(),
                color = PineGlowMist,
                fontSize = 13.sp,
                fontWeight = FontWeight.Medium,
            )
        }
    }
}
