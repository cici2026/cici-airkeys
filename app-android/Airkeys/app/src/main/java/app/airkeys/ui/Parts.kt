package app.airkeys.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.contextmenu.data.TextContextMenuKeys
import androidx.compose.foundation.text.contextmenu.modifier.filterTextContextMenuComponents
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.InterceptPlatformTextInput
import androidx.compose.ui.platform.LocalTextToolbar
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.TextToolbar
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.airkeys.R
import app.airkeys.core.NetworkKind
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** A screen: a top bar, scrolling content, and actions pinned at the bottom. */
/** The side margin of every page. */
val PAGE_MARGIN = 20.dp

@Composable
fun Page(
    title: String? = null,
    onBack: (() -> Unit)? = null,
    trailing: @Composable RowScope.() -> Unit = {},
    actions: @Composable ColumnScope.() -> Unit = {},
    scroll: Boolean = true,
    content: @Composable ColumnScope.() -> Unit,
) {
    Column(Modifier.fillMaxSize().background(Ink.bg).systemBarsPadding().imePadding()) {
        Row(Modifier.fillMaxWidth().height(56.dp).padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            if (onBack != null) IconAction(Icons.back, tr("Back", "返回"), onBack) else Spacer(Modifier.width(12.dp))
            Text(title ?: "", style = Type.heading.copy(fontSize = 17.sp), modifier = Modifier.weight(1f).padding(start = 4.dp), maxLines = 1)
            trailing()
        }
        val body = Modifier.weight(1f).fillMaxWidth().let { if (scroll) it.verticalScroll(rememberScrollState()) else it }
        Column(body.padding(horizontal = PAGE_MARGIN), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Spacer(Modifier.height(4.dp))
            content()
            Spacer(Modifier.height(12.dp))
        }
        Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) { actions() }
    }
}

@Composable
fun IconAction(icon: ImageVector, label: String, onClick: () -> Unit, tint: Color = Ink.text) {
    Box(Modifier.size(44.dp).clip(CircleShape).clickable(onClick = onClick), contentAlignment = Alignment.Center) {
        Icon(icon, label, tint = tint, modifier = Modifier.size(22.dp))
    }
}

@Composable
private fun ButtonBase(text: String, bg: Color, fg: Color, border: Color?, enabled: Boolean, onClick: () -> Unit, modifier: Modifier, icon: ImageVector?) {
    val shape = RoundedCornerShape(14.dp)
    Row(
        modifier.fillMaxWidth().height(54.dp).clip(shape)
            // disabled: a neutral surface, not a faded colour
            .background(if (enabled) bg else Ink.raised)
            .let { if (border != null) it.border(1.dp, if (enabled) border else Ink.stroke, shape) else it }
            .clickable(enabled = enabled, onClick = onClick),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            Icon(icon, null, tint = if (enabled) fg else Ink.faint, modifier = Modifier.size(20.dp))
            Spacer(Modifier.width(10.dp))
        }
        Text(text, style = TextStyle(fontSize = 16.sp, fontWeight = FontWeight.SemiBold, color = if (enabled) fg else Ink.faint))
    }
}

@Composable
fun PrimaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, icon: ImageVector? = null) =
    ButtonBase(text, Ink.lamp, Ink.onLamp, null, enabled, onClick, modifier, icon)

@Composable
fun SecondaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, icon: ImageVector? = null) =
    ButtonBase(text, Ink.raised, Ink.text, Ink.stroke, enabled, onClick, modifier, icon)

private const val HOLD_MS = 900

/**
 * Press and hold to confirm: an accidental tap cannot sign. The bar fills while held.
 */
@Composable
fun HoldButton(text: String, onConfirm: () -> Unit, warn: Boolean) {
    val progress = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()
    val shape = RoundedCornerShape(14.dp)
    val color = if (warn) Ink.warn else Ink.lamp
    Box(
        Modifier.fillMaxWidth().height(58.dp).clip(shape).background(color)
            .pointerInput(Unit) {
                awaitEachGesture {
                    awaitFirstDown()
                    // the hold is timed by the clock, not the animation: with system
                    // animations off, an animation ends at once and a tap would sign
                    var held = false
                    val job = scope.launch {
                        launch {
                            progress.snapTo(0f)
                            progress.animateTo(1f, tween(HOLD_MS, easing = LinearEasing))
                        }
                        delay(HOLD_MS.toLong())
                        held = true
                        onConfirm()
                    }
                    waitForUpOrCancellation()
                    if (!held) {
                        job.cancel()
                        scope.launch { progress.animateTo(0f, tween(180)) }
                    }
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        // a primary button; while held, a light band fills it
        Box(Modifier.matchParentSize()) {
            Box(Modifier.fillMaxWidth(progress.value).height(58.dp).background(Color.White.copy(alpha = 0.32f)))
        }
        Text(text, style = TextStyle(fontSize = 16.sp, fontWeight = FontWeight.SemiBold, color = Ink.onLamp))
    }
}

@Composable
fun Card(modifier: Modifier = Modifier, border: Color = Ink.stroke, padding: Dp = 16.dp, content: @Composable ColumnScope.() -> Unit) {
    val shape = RoundedCornerShape(16.dp)
    Column(
        modifier.fillMaxWidth().clip(shape).background(Ink.surface).border(1.dp, border, shape).padding(padding),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        content = content,
    )
}

/**
 * An address in groups of four, the first and last groups bright: how people compare
 * addresses, so the layout helps the check rather than hiding it.
 */
@Composable
fun Address(address: String, modifier: Modifier = Modifier, style: TextStyle = Type.mono) {
    // EVM addresses: "0x" apart, then the 40 digits in tens of fours
    val prefix = if (address.startsWith("0x")) "0x" else ""
    val groups = address.removePrefix(prefix).chunked(4)
    Text(
        buildAnnotatedString {
            if (prefix.isNotEmpty()) withStyle(SpanStyle(color = Ink.faint)) { append("0x ") }
            // one weight; the first and last groups brighter, as people compare them
            groups.forEachIndexed { i, g ->
                val edge = i == 0 || i == groups.size - 1
                withStyle(SpanStyle(color = if (edge) Ink.text else Ink.dim)) { append(g) }
                if (i < groups.size - 1) append(" ")
            }
        },
        style = style,
        modifier = modifier,
    )
}

/** First and last characters with an ellipsis, for lists. */
fun short(address: String, keep: Int = 6) = if (address.length <= keep * 2 + 1) address else "${address.take(keep + if (address.startsWith("0x")) 2 else 0)}…${address.takeLast(keep)}"

/** The network's own logo. */
@Composable
fun NetworkBadge(kind: NetworkKind, size: Dp = 40.dp) {
    val logo = when (kind) {
        NetworkKind.BITCOIN -> R.drawable.logo_btc
        NetworkKind.EVM -> R.drawable.logo_eth
    }
    Image(painterResource(logo), null, Modifier.size(size))
}

@Composable
fun Pill(text: String, color: Color, icon: ImageVector? = null) {
    Row(
        Modifier.clip(RoundedCornerShape(50)).background(color.copy(alpha = 0.13f)).padding(horizontal = 10.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            Icon(icon, null, tint = color, modifier = Modifier.size(14.dp))
            Spacer(Modifier.width(5.dp))
        }
        Text(text, style = TextStyle(fontSize = 12.sp, fontWeight = FontWeight.SemiBold, color = color))
    }
}

/** A message in a tinted card: a warning, a fact, a problem. */
@Composable
fun Notice(text: String, color: Color, icon: ImageVector = Icons.warning) {
    Row(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(14.dp)).background(color.copy(alpha = 0.10f)).border(1.dp, color.copy(alpha = 0.32f), RoundedCornerShape(14.dp)).padding(14.dp),
        verticalAlignment = Alignment.Top,
    ) {
        Icon(icon, null, tint = color, modifier = Modifier.size(20.dp))
        Spacer(Modifier.width(12.dp))
        Text(text, style = Type.body.copy(fontSize = 14.sp, lineHeight = 20.sp, color = Ink.text.copy(alpha = 0.92f)))
    }
}

/** The Airkeys mark: a keyhole inside a ring. */
@Composable
fun Mark(size: Dp = 72.dp, color: Color = Ink.lamp) {
    Canvas(Modifier.size(size)) {
        val s = this.size.minDimension / 54f
        drawCircle(color, radius = 24f * s, style = Stroke(width = 4f * s))
        val p = Path().apply {
            addOval(Rect(Offset(27f * s - 8f * s, 13.5f * s), Size(16f * s, 16f * s)))
            moveTo(24.4f * s, 26f * s)
            lineTo(21.2f * s, 40f * s)
            lineTo(32.8f * s, 40f * s)
            lineTo(29.6f * s, 26f * s)
            close()
        }
        drawPath(p, color)
    }
}

/** A range of a field's text to select; `stamp` makes the same range a new request. */
data class Selection(val range: TextRange, val stamp: Long = System.nanoTime())

/**
 * A text field for secrets: no autocorrect, no suggestions, no learning — the keyboard must not
 * remember a recovery phrase — and nothing to copy or cut out of it.
 */
@Composable
fun Field(
    value: String,
    onChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    secret: Boolean = false,
    singleLine: Boolean = true,
    minLines: Int = 1,
    error: Boolean = false,
    rewrite: ((String) -> String)? = null,
    select: Selection? = null,
) {
    val shape = RoundedCornerShape(14.dp)
    val focus = remember { FocusRequester() }
    // the core may rewrite the text (a completed word): then the cursor goes to the end;
    // otherwise it stays where the person put it
    var field by remember { mutableStateOf(TextFieldValue(value, TextRange(value.length))) }
    if (field.text != value) field = TextFieldValue(value, TextRange(value.length))
    // a part of the text chosen elsewhere (a word tapped in the grid): selected, so typing replaces it
    LaunchedEffect(select) {
        val range = select?.range ?: return@LaunchedEffect
        if (range.max <= field.text.length) field = field.copy(selection = range)
        focus.requestFocus()
    }
    Private {
        BasicTextField(
            value = field,
            onValueChange = {
                if (rewrite != null) {
                    // the core's text, at once: a second keystroke cannot land on stale text
                    val canonical = if (it.text != field.text) rewrite(it.text) else it.text
                    field = if (canonical == it.text) it else TextFieldValue(canonical, TextRange(canonical.length))
                } else {
                    field = it
                    if (it.text != value) onChange(it.text)
                }
            },
            singleLine = singleLine,
            minLines = minLines,
            textStyle = Type.mono.copy(fontSize = 17.sp, lineHeight = 26.sp),
            cursorBrush = SolidColor(Ink.lamp),
            visualTransformation = if (secret) PasswordVisualTransformation() else VisualTransformation.None,
            keyboardOptions = KeyboardOptions(
                capitalization = KeyboardCapitalization.None,
                autoCorrectEnabled = false,
                // visible or not, a password field to the keyboard: no suggestions, no learning
                keyboardType = KeyboardType.Password,
            ),
            modifier = modifier.focusRequester(focus).fillMaxWidth().clip(shape).background(Ink.surface)
                .border(BorderStroke(1.dp, if (error) Ink.danger else Ink.stroke), shape).padding(16.dp)
                .filterTextContextMenuComponents { it.key != TextContextMenuKeys.CopyKey && it.key != TextContextMenuKeys.CutKey },
            decorationBox = { inner ->
                Box {
                    if (value.isEmpty()) Text(placeholder, style = Type.mono.copy(fontSize = 17.sp, color = Ink.faint))
                    inner()
                }
            },
        )
    }
}

/**
 * Text input below it that keyboards must not learn from (Android's no-personalized-learning
 * flag; a password type alone is only a hint), and whose menu offers no Copy or Cut.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun Private(content: @Composable () -> Unit) {
    val platform = LocalTextToolbar.current
    // the older text menu; the newer one is filtered on the field itself
    val toolbar = remember(platform) {
        object : TextToolbar by platform {
            override fun showMenu(
                rect: Rect,
                onCopyRequested: (() -> Unit)?,
                onPasteRequested: (() -> Unit)?,
                onCutRequested: (() -> Unit)?,
                onSelectAllRequested: (() -> Unit)?,
            ) = platform.showMenu(rect, null, onPasteRequested, null, onSelectAllRequested)

            override fun showMenu(
                rect: Rect,
                onCopyRequested: (() -> Unit)?,
                onPasteRequested: (() -> Unit)?,
                onCutRequested: (() -> Unit)?,
                onSelectAllRequested: (() -> Unit)?,
                onAutofillRequested: (() -> Unit)?,
            ) = platform.showMenu(rect, null, onPasteRequested, null, onSelectAllRequested, null)
        }
    }
    CompositionLocalProvider(LocalTextToolbar provides toolbar) {
        InterceptPlatformTextInput(
            interceptor = { request, next ->
                next.startInputMethod(object : PlatformTextInputMethodRequest {
                    override fun createInputConnection(outAttributes: EditorInfo): InputConnection {
                        val connection = request.createInputConnection(outAttributes)
                        outAttributes.imeOptions = outAttributes.imeOptions or EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                        return connection
                    }
                })
            },
            content = content,
        )
    }
}

@Composable
fun Suggestions(words: List<String>, onPick: (String) -> Unit) {
    if (words.isEmpty()) return
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (w in words) {
            Box(
                Modifier.clip(RoundedCornerShape(10.dp)).background(Ink.raised).border(1.dp, Ink.stroke, RoundedCornerShape(10.dp))
                    .clickable { onPick(w) }.padding(horizontal = 14.dp, vertical = 9.dp),
            ) { Text(w, style = Type.mono.copy(fontSize = 15.sp)) }
        }
    }
}

@Composable
fun Center(content: @Composable ColumnScope.() -> Unit) {
    Column(
        Modifier.fillMaxWidth().padding(top = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(14.dp),
        content = content,
    )
}

@Composable
fun CenteredText(text: String, style: TextStyle) = Text(text, style = style, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())

@Composable
fun Label(text: String) = Text(text.uppercase(), style = Type.label)

