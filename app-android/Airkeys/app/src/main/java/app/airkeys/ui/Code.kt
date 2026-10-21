package app.airkeys.ui

import android.graphics.Bitmap
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import app.airkeys.core.CodeFrame
import app.airkeys.core.codeFps
import kotlin.math.floor

/**
 * The animated code, as large as the phone allows: the whole width of the screen, edge to edge,
 * and the height it is given. The core shapes the code to this area (a tall code on a phone).
 * One pixel per cell from the core, scaled by a whole number with nearest-neighbour filtering,
 * so every cell edge stays sharp. Big cells are what a camera reads fastest, and from farther
 * away; filling the screen carries more in each frame.
 */
@Composable
fun CodeView(next: (Int, Int) -> CodeFrame?, modifier: Modifier = Modifier) {
    var image by remember { mutableStateOf<ImageBitmap?>(null) }
    var area by remember { mutableStateOf(IntSize.Zero) }
    LaunchedEffect(Unit) {
        val interval = 1_000_000_000L / codeFps().toLong()
        var due = 0L
        // on the display's frames: a steady code_fps, and nothing while the app is out of sight
        while (true) {
            withFrameNanos { now ->
                if (now < due) return@withFrameNanos
                due = if (now - due > interval) now + interval else due + interval
                val a = area
                if (a.width > 0 && a.height > 0) next(a.width, a.height)?.let { image = it.toBitmap().asImageBitmap() }
            }
        }
    }
    Box(
        modifier
            // out of the page's side margins: the whole width of the screen
            .layout { m, c ->
                val wide = c.maxWidth + 2 * PAGE_MARGIN.roundToPx()
                val p = m.measure(c.copy(minWidth = wide, maxWidth = wide))
                layout(c.maxWidth, p.height) { p.place(-PAGE_MARGIN.roundToPx(), 0) }
            }
            .onSizeChanged { area = it }
            .background(Color(0xFF0A0D14)), // the code's own quiet zone (Lumen's ink)
    ) {
        val img = image ?: return@Box
        Canvas(Modifier.fillMaxSize()) {
            val scale = floor(minOf(size.width / img.width, size.height / img.height)).coerceAtLeast(1f)
            val w = (img.width * scale).toInt()
            val h = (img.height * scale).toInt()
            drawImage(
                img,
                dstOffset = IntOffset(((size.width - w) / 2).toInt(), ((size.height - h) / 2).toInt()),
                dstSize = IntSize(w, h),
                filterQuality = FilterQuality.None,
            )
        }
    }
}

private fun CodeFrame.toBitmap(): Bitmap {
    val w = width.toInt()
    val h = height.toInt()
    val px = IntArray(w * h)
    for (i in px.indices) {
        val r = rgba[i * 4].toInt() and 255
        val g = rgba[i * 4 + 1].toInt() and 255
        val b = rgba[i * 4 + 2].toInt() and 255
        px[i] = (0xFF shl 24) or (r shl 16) or (g shl 8) or b
    }
    return Bitmap.createBitmap(px, w, h, Bitmap.Config.ARGB_8888)
}
