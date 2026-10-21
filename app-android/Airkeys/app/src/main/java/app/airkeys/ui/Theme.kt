package app.airkeys.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import app.airkeys.core.NetworkKind

/** The Airkeys palette: a dark bunker, one warm lamp. Same values on iOS and the web. */
object Ink {
    val bg = Color(0xFF0B0D11)
    val surface = Color(0xFF13161C)
    val raised = Color(0xFF1A1E26)
    val stroke = Color(0xFF262B35)
    val text = Color(0xFFECEEF2)
    val dim = Color(0xFF9BA3B4)
    val faint = Color(0xFF687082)
    val lamp = Color(0xFFF2B544)
    val onLamp = Color(0xFF1C1505)
    val safe = Color(0xFF3FD08F)
    val warn = Color(0xFFF5A524)
    val danger = Color(0xFFF2555A)

    fun network(kind: NetworkKind): Color = when (kind) {
        NetworkKind.BITCOIN -> Color(0xFFF7931A)
        NetworkKind.EVM -> Color(0xFF627EEA)
    }
}

object Type {
    val title = TextStyle(fontSize = 28.sp, lineHeight = 34.sp, fontWeight = FontWeight.SemiBold, color = Ink.text, letterSpacing = (-0.3).sp)
    val heading = TextStyle(fontSize = 20.sp, lineHeight = 26.sp, fontWeight = FontWeight.SemiBold, color = Ink.text)
    val body = TextStyle(fontSize = 16.sp, lineHeight = 23.sp, color = Ink.text)
    val dim = TextStyle(fontSize = 15.sp, lineHeight = 21.sp, color = Ink.dim)
    val small = TextStyle(fontSize = 13.sp, lineHeight = 18.sp, color = Ink.dim)
    val label = TextStyle(fontSize = 12.sp, lineHeight = 16.sp, fontWeight = FontWeight.SemiBold, color = Ink.faint, letterSpacing = 0.8.sp)
    val mono = TextStyle(fontSize = 15.sp, lineHeight = 22.sp, fontFamily = FontFamily.Monospace, color = Ink.text)
    val amount = TextStyle(fontSize = 30.sp, lineHeight = 36.sp, fontWeight = FontWeight.SemiBold, color = Ink.text, letterSpacing = (-0.4).sp)
}

@Composable
fun AirkeysTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = Ink.lamp,
            onPrimary = Ink.onLamp,
            background = Ink.bg,
            surface = Ink.surface,
            onSurface = Ink.text,
            onBackground = Ink.text,
            outline = Ink.stroke,
            error = Ink.danger,
        ),
        content = content,
    )
}
