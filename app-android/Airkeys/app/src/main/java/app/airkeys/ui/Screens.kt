package app.airkeys.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.text.TextRange
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.platform.LocalContext
import android.content.Context
import android.content.Intent
import android.provider.Settings
import app.airkeys.core.AccountRow
import app.airkeys.core.Action
import app.airkeys.core.CodeFrame
import app.airkeys.core.NetworkKind
import app.airkeys.core.accountCount
import app.airkeys.core.PhraseInput
import app.airkeys.core.RadioState
import app.airkeys.core.ReviewView
import app.airkeys.core.Screen
import app.airkeys.core.ViewModel
import app.airkeys.core.WordState
import kotlinx.coroutines.delay

/** Draws the core's view; every tap becomes an action for the core. */
@Composable
fun App(view: ViewModel, act: (Action) -> ViewModel, codeFrame: (Int, Int) -> CodeFrame?, camera: @Composable () -> Unit) {
    Box(Modifier.fillMaxSize().background(Ink.bg)) {
        when (val s = view.screen) {
            is Screen.Offline -> Offline(s)
            is Screen.Welcome -> Welcome(view.version, act)
            is Screen.Import -> Import(s.phrase, act)
            is Screen.Passphrase -> Passphrase(s, act)
            is Screen.Home -> Home(s, act)
            is Screen.Account -> AccountScreen(s.account, act)
            is Screen.Connect -> Page("Connect", onBack = { act(Action.Back) }, actions = {
                PrimaryButton("Show code", { act(Action.ShowAccounts) }, enabled = s.choices.any { it.selected }, icon = Icons.code)
            }) {
                Text("Show your accounts to the web app", style = Type.title)
                Text("The web app reads balances and prepares transfers. It gets addresses only — never a public key.", style = Type.dim)
                Card(padding = 6.dp) {
                    for (c in s.scopes) {
                        Row(
                            Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).clickable { act(Action.ConnectScope(c.all)) }.padding(10.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(Modifier.weight(1f).padding(end = 12.dp)) {
                                Text(c.title, style = Type.body.copy(fontWeight = FontWeight.SemiBold))
                                Text(c.detail, style = Type.small)
                            }
                            Radio(c.selected)
                        }
                    }
                }
                Card(padding = 6.dp) {
                    for (c in s.choices) {
                        Row(
                            Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).clickable { act(Action.ToggleNetwork(c.network)) }.padding(10.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            NetworkBadge(c.network, 36.dp)
                            Text(c.name, style = Type.body, modifier = Modifier.weight(1f).padding(start = 14.dp))
                            Check(c.selected)
                        }
                    }
                }
            }
            is Screen.Code -> {
                // the code fills the screen from the title to the button; its message is a tap away
                var reading by remember(s.json) { mutableStateOf(false) }
                Page(s.title, onBack = { act(Action.Back) }, scroll = reading, trailing = {
                    Text(
                        if (reading) "Code" else "Message",
                        style = Type.body.copy(color = Ink.lamp, fontWeight = FontWeight.SemiBold),
                        modifier = Modifier.clip(RoundedCornerShape(10.dp)).clickable { reading = !reading }.padding(horizontal = 12.dp, vertical = 10.dp),
                    )
                }, actions = { PrimaryButton("Done", { act(Action.Done) }) }) {
                    if (reading) {
                        Text("What the code says", style = Type.heading)
                        Card { Text(s.json, style = Type.mono.copy(fontSize = 12.sp, lineHeight = 17.sp, color = Ink.dim)) }
                    } else {
                        CodeView(codeFrame, Modifier.weight(1f))
                        Text(s.caption, style = Type.dim, maxLines = 2)
                    }
                }
            }
            is Screen.Scan -> Page(s.title, onBack = { act(Action.Back) }, scroll = false, actions = {
                if (s.problem != null) PrimaryButton("Try again", { act(Action.Scan) }, icon = Icons.scan)
            }) {
                Box(Modifier.fillMaxWidth().aspectRatio(0.8f).clip(RoundedCornerShape(20.dp)).background(Color(0xFF05060A))) {
                    if (s.problem == null) camera()
                    Viewfinder()
                }
                val p = s.progress
                when {
                    s.problem != null -> Notice(s.problem!!, Ink.danger)
                    p != null && p.needed > 0u -> {
                        Text("Receiving…", style = Type.heading)
                        LinearProgressIndicator(
                            progress = { (p.received.toFloat() / p.needed.toFloat()).coerceIn(0f, 1f) },
                            modifier = Modifier.fillMaxWidth().height(6.dp).clip(RoundedCornerShape(3.dp)),
                            color = Ink.lamp,
                            trackColor = Ink.raised,
                        )
                    }
                    else -> Text(s.hint, style = Type.dim)
                }
            }
            is Screen.Review -> Review(s.review, act)
            is Screen.Refused -> Page(onBack = { act(Action.Back) }, actions = { SecondaryButton("Back", { act(Action.Done) }) }) {
                Center {
                    Icon(Icons.warning, null, tint = Ink.danger, modifier = Modifier.size(52.dp))
                    CenteredText(s.title, Type.title.copy(fontSize = 24.sp))
                }
                Card { Text(s.message, style = Type.body) }
            }
        }
        Banner(view, act)
    }
}

@Composable
private fun Banner(view: ViewModel, act: (Action) -> ViewModel) {
    val toast = view.toast
    LaunchedEffect(toast) {
        if (toast != null) {
            delay(5000)
            act(Action.DismissToast)
        }
    }

    Column(Modifier.fillMaxSize().navigationBarsPadding().padding(16.dp), verticalArrangement = Arrangement.Bottom) {
        AnimatedVisibility(toast != null, enter = fadeIn(), exit = fadeOut()) {
            Row(
                Modifier.fillMaxWidth().padding(bottom = 76.dp).clip(RoundedCornerShape(14.dp)).background(Ink.raised).border(1.dp, Ink.stroke, RoundedCornerShape(14.dp))
                    .clickable { act(Action.DismissToast) }.padding(16.dp),
            ) { Text(toast ?: "", style = Type.body.copy(fontSize = 15.sp)) }
        }
    }
}

@Composable
private fun Offline(s: Screen.Offline) {
    Page {
        Spacer(Modifier.height(24.dp))
        Box(Modifier.size(64.dp).clip(RoundedCornerShape(20.dp)).background(Ink.warn.copy(alpha = 0.14f)), contentAlignment = Alignment.Center) {
            Icon(Icons.airplane, null, tint = Ink.warn, modifier = Modifier.size(32.dp))
        }
        Text("Take this phone offline", style = Type.title)
        Text(
            "Airkeys works only on a phone with every radio off. Turn on Airplane mode, then make sure Wi-Fi and Bluetooth stay off.",
            style = Type.dim,
        )
        val context = LocalContext.current
        Card(padding = 6.dp) {
            for ((i, r) in s.radios.withIndex()) {
                Column(Modifier.fillMaxWidth().padding(12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(r.name, style = Type.body, modifier = Modifier.weight(1f))
                        when (r.state) {
                            RadioState.OFF -> Pill("Off", Ink.safe, Icons.check)
                            RadioState.ON -> Pill("On", Ink.warn, Icons.warning)
                            RadioState.ACTIVE -> Pill("Still active", Ink.warn, Icons.warning)
                            RadioState.UNKNOWN -> Pill("Unknown", Ink.dim)
                        }
                    }
                    r.detail?.let { Text(it, style = Type.small, modifier = Modifier.padding(top = 4.dp, end = 4.dp)) }
                    if (r.state == RadioState.ON || r.state == RadioState.ACTIVE) {
                        Text(
                            "Open settings",
                            style = Type.small.copy(color = Ink.lamp, fontWeight = FontWeight.SemiBold),
                            modifier = Modifier.padding(top = 6.dp).clip(RoundedCornerShape(8.dp))
                                .clickable { openRadioSettings(context, i, r.state) }.padding(vertical = 4.dp),
                        )
                    }
                }
            }
        }
        if (s.walletOpen) Notice("Your wallet is still open. Airkeys carries on as soon as the phone is offline again.", Ink.dim, Icons.lock)
    }
}

/**
 * The system screen that turns a radio off, in the order of the core's rows: Wi-Fi, Bluetooth,
 * mobile data, network. A radio that is "still active" while switched off is scanning for
 * Location: its switch is under Location's scanning settings. Phones that lack a screen get the
 * next best one.
 */
private fun openRadioSettings(context: Context, row: Int, state: RadioState) {
    val scanning = listOf("android.settings.LOCATION_SCANNING_SETTINGS", Settings.ACTION_LOCATION_SOURCE_SETTINGS)
    val tries = when (row) {
        0 -> if (state == RadioState.ACTIVE) listOf("android.settings.WIFI_SCANNING_SETTINGS") + scanning else listOf(Settings.ACTION_WIFI_SETTINGS)
        1 -> if (state == RadioState.ACTIVE) listOf("android.settings.BLUETOOTH_SCANNING_SETTINGS") + scanning else listOf(Settings.ACTION_BLUETOOTH_SETTINGS)
        2 -> listOf(Settings.ACTION_AIRPLANE_MODE_SETTINGS)
        else -> listOf(Settings.ACTION_WIRELESS_SETTINGS)
    } + Settings.ACTION_SETTINGS
    for (action in tries) {
        if (runCatching { context.startActivity(Intent(action)) }.isSuccess) return
    }
}

@Composable
private fun Welcome(version: String, act: (Action) -> ViewModel) {
    Page(actions = {
        PrimaryButton("Enter recovery phrase", { act(Action.Begin) })
        SecondaryButton("Scan SeedQR", { act(Action.Scan) }, icon = Icons.scan)
    }) {
        Spacer(Modifier.height(24.dp))
        Mark(76.dp)
        Text("Airkeys", style = Type.title.copy(fontSize = 38.sp, lineHeight = 44.sp))
        Text(
            "A cold wallet for a phone that stays offline. It shows the world addresses — never a public key — so the funds behind them stay out of reach, even of a quantum computer.",
            style = Type.dim.copy(fontSize = 16.sp, lineHeight = 23.sp),
        )
        Spacer(Modifier.height(8.dp))
        Feature(Icons.lock, "Nothing is stored", "Bring your recovery phrase each time. It stays in memory until you close the wallet or the app.")
        Feature(Icons.shield, "Addresses only", "Connecting to the web app shares addresses, never the keys behind them.")
        Feature(Icons.eye, "Clear signing", "Airkeys signs transfers only, after showing every recipient, amount and fee.")
        Text(version, style = Type.small.copy(color = Ink.faint))
    }
}

@Composable
private fun Feature(icon: ImageVector, title: String, text: String) {
    Row(verticalAlignment = Alignment.Top) {
        Box(Modifier.size(38.dp).clip(RoundedCornerShape(11.dp)).background(Ink.lamp.copy(alpha = 0.12f)), contentAlignment = Alignment.Center) {
            Icon(icon, null, tint = Ink.lamp, modifier = Modifier.size(20.dp))
        }
        Column(Modifier.padding(start = 14.dp)) {
            Text(title, style = Type.body.copy(fontWeight = FontWeight.SemiBold))
            Text(text, style = Type.small.copy(fontSize = 14.sp, lineHeight = 19.sp))
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun Import(p: PhraseInput, act: (Action) -> ViewModel) {
    val focus = remember { FocusRequester() }
    // a word tapped in the grid is selected in the field: typing replaces it
    var select by remember { mutableStateOf<Selection?>(null) }
    val spans = Regex("\\S+").findAll(p.text).map { TextRange(it.range.first, it.range.last + 1) }.toList()
    LaunchedEffect(Unit) { focus.requestFocus() }
    Page("Recovery phrase", onBack = { act(Action.Back) }, trailing = {
        IconAction(Icons.scan, "Scan SeedQR", { act(Action.Scan) }, Ink.dim)
    }, actions = {
        PrimaryButton("Continue", { act(Action.PhraseDone) }, enabled = p.words.size >= 12 && p.problem == null)
    }) {
        Text("Enter your recovery phrase", style = Type.title)
        Text("Type the first letters of each word and a space: Airkeys completes it.", style = Type.dim)
        Field(
            p.text, {}, "abandon ability …", Modifier.focusRequester(focus), singleLine = false, minLines = 3, error = p.problem != null,
            rewrite = { t -> (act(Action.PhraseInput(t)).screen as? Screen.Import)?.phrase?.text ?: t },
            select = select,
        )
        Suggestions(p.suggestions) { act(Action.Suggestion(it)) }
        p.problem?.let { Text(it, style = Type.small.copy(color = Ink.danger)) }
        if (p.words.isNotEmpty()) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Label("${p.words.size} words")
                Spacer(Modifier.weight(1f))
                if (p.valid) Pill("Valid phrase", Ink.safe, Icons.check)
            }
            // as on a recovery card: even columns, numbers aligned
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                for (row in p.words.indices.chunked(3)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        for (k in 0 until 3) {
                            val i = row.getOrNull(k)
                            if (i == null) {
                                Spacer(Modifier.weight(1f))
                                continue
                            }
                            val w = p.words[i]
                            val shape = RoundedCornerShape(10.dp)
                            val chip = when (w.state) {
                                WordState.KNOWN -> Modifier.background(Ink.raised, shape)
                                WordState.TYPING -> Modifier.border(1.dp, Ink.stroke, shape)
                                WordState.UNKNOWN -> Modifier.background(Ink.danger.copy(alpha = 0.12f), shape)
                            }
                            Row(
                                Modifier.weight(1f).clip(shape).then(chip)
                                    .clickable { spans.getOrNull(i)?.let { select = Selection(it) } }
                                    .padding(horizontal = 8.dp, vertical = 8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text("${i + 1}", style = Type.mono.copy(fontSize = 12.sp, color = Ink.faint, textAlign = TextAlign.End), modifier = Modifier.width(18.dp))
                                Spacer(Modifier.width(6.dp))
                                Text(
                                    w.word,
                                    style = Type.mono.copy(fontSize = 15.sp, color = when (w.state) {
                                        WordState.KNOWN -> Ink.text
                                        WordState.TYPING -> Ink.dim
                                        WordState.UNKNOWN -> Ink.danger
                                    }),
                                    maxLines = 1,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun Passphrase(s: Screen.Passphrase, act: (Action) -> ViewModel) {
    Page("Passphrase", onBack = { act(Action.Back) }, actions = {
        PrimaryButton("Open wallet", { act(Action.Open) }, enabled = s.problem == null)
    }) {
        Text("Passphrase", style = Type.title)
        Text("Only if your wallet has one — most do not. Leave it empty otherwise.", style = Type.dim)
        Field(s.passphrase, { act(Action.PassphraseInput(it)) }, "No passphrase", secret = true, error = s.problem != null)
        s.problem?.let { Text(it, style = Type.small.copy(color = Ink.danger)) }
        Notice("A passphrase opens a different wallet from the same words, and any passphrase opens one. Airkeys cannot tell a mistyped passphrase: check the addresses it shows.", Ink.dim, Icons.lock)
    }
}

@Composable
private fun Home(s: Screen.Home, act: (Action) -> ViewModel) {
    Page(
        trailing = {
            Row(
                Modifier.clip(RoundedCornerShape(50)).clickable { act(Action.Close) }.padding(horizontal = 12.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(Icons.lock, null, tint = Ink.dim, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(6.dp))
                Text("Close wallet", style = Type.small.copy(color = Ink.dim))
            }
        },
        actions = {
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                SecondaryButton("Connect", { act(Action.Connect) }, Modifier.weight(1f), icon = Icons.code)
                PrimaryButton("Scan request", { act(Action.Scan) }, Modifier.weight(1.4f), icon = Icons.scan)
            }
        },
    ) {
        NumberWheel(s.index.toInt(), accountCount().toInt(), { act(Action.Account(it.toUInt())) }, unit = "Account")
        for (a in s.accounts) AccountCard(a) { act(Action.OpenAccount(a.network)) }
    }
}

@Composable
private fun AccountCard(a: AccountRow, onClick: () -> Unit) {
    Card(Modifier.clickable(onClick = onClick)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            NetworkBadge(a.network)
            Column(Modifier.weight(1f).padding(start = 14.dp)) {
                Text(a.name, style = Type.body.copy(fontWeight = FontWeight.SemiBold))
                Text(a.assets, style = Type.small, maxLines = 1)
            }
        }
        Text(short(a.address, 8), style = Type.mono.copy(fontSize = 14.sp, color = Ink.dim))
    }
}

@Composable
private fun AccountScreen(a: AccountRow, act: (Action) -> ViewModel) {
    Page(a.name, onBack = { act(Action.Back) }, actions = {
        PrimaryButton("Show to the web app", { act(Action.ShareAccount(a.network)) }, icon = Icons.code)
    }) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            NetworkBadge(a.network, 52.dp)
            Column(Modifier.padding(start = 14.dp).weight(1f)) {
                Text("Account #${a.index}", style = Type.heading)
                Text(a.assets, style = Type.small)
            }
        }
        Card {
            Label("Address")
            Address(a.address, style = Type.mono.copy(fontSize = 19.sp, lineHeight = 30.sp))
        }
        Card {
            Label("Derivation path")
            Text(a.path, style = Type.mono)
            Text("Every step is hardened.", style = Type.small)
        }
        Notice("The public key behind this address stays on this phone until the account first sends. Receiving reveals nothing.", Ink.safe, Icons.shield)
    }
}

@Composable
private fun Radio(on: Boolean) {
    Box(
        Modifier.size(24.dp).clip(CircleShape).border(2.dp, if (on) Ink.lamp else Ink.stroke, CircleShape),
        contentAlignment = Alignment.Center,
    ) { if (on) Box(Modifier.size(12.dp).clip(CircleShape).background(Ink.lamp)) }
}

@Composable
private fun Check(on: Boolean) {
    Box(
        Modifier.size(26.dp).clip(RoundedCornerShape(8.dp)).background(if (on) Ink.lamp else Ink.raised).border(1.dp, if (on) Ink.lamp else Ink.stroke, RoundedCornerShape(8.dp)),
        contentAlignment = Alignment.Center,
    ) { if (on) Icon(Icons.check, null, tint = Ink.onLamp, modifier = Modifier.size(18.dp)) }
}

@Composable
private fun Viewfinder() {
    androidx.compose.foundation.Canvas(Modifier.fillMaxSize().padding(28.dp)) {
        val l = size.minDimension * 0.12f
        val w = 3.dp.toPx()
        val c = Ink.lamp
        val (x0, y0, x1, y1) = listOf(0f, 0f, size.width, size.height)
        for ((x, y, dx, dy) in listOf(listOf(x0, y0, 1f, 1f), listOf(x1, y0, -1f, 1f), listOf(x0, y1, 1f, -1f), listOf(x1, y1, -1f, -1f))) {
            drawLine(c, androidx.compose.ui.geometry.Offset(x, y), androidx.compose.ui.geometry.Offset(x + dx * l, y), w, androidx.compose.ui.graphics.StrokeCap.Round)
            drawLine(c, androidx.compose.ui.geometry.Offset(x, y), androidx.compose.ui.geometry.Offset(x, y + dy * l), w, androidx.compose.ui.graphics.StrokeCap.Round)
        }
    }
}

@Composable
private fun Review(r: ReviewView, act: (Action) -> ViewModel) {
    var details by remember { mutableStateOf(false) }
    Page("Review", onBack = { act(Action.Reject) }, actions = {
        HoldButton("Hold to ${r.signLabel.lowercase()}", { act(Action.Sign) }, warn = r.warnings.isNotEmpty())
        SecondaryButton("Reject", { act(Action.Reject) })
    }) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            NetworkBadge(r.network, 44.dp)
            Column(Modifier.padding(start = 14.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(r.chain, style = Type.heading)
                    // test coins have no value: never mistake one network for the other
                    if (r.testnet) Spacer(Modifier.width(10.dp))
                    if (r.testnet) Pill("Testnet", Ink.warn)
                }
                Text("From account #${r.fromIndex} · ${short(r.fromAddress)}", style = Type.small)
            }
        }
        for (w in r.warnings) Notice(w, Ink.warn)
        r.transfers.forEachIndexed { i, t ->
            Card(border = if (!t.verified) Ink.warn.copy(alpha = 0.5f) else Ink.stroke) {
                if (r.transfers.size > 1 && r.network != NetworkKind.BITCOIN) Label("Transaction ${i + 1}")
                Text("${t.amount} ${t.asset}", style = Type.amount)
                t.token?.let {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (t.verified) Pill("Listed token", Ink.safe, Icons.check) else Pill("Unlisted token", Ink.warn, Icons.warning)
                        Text(short(it, 6), style = Type.mono.copy(fontSize = 13.sp, color = Ink.dim), modifier = Modifier.padding(start = 10.dp))
                    }
                }
                Spacer(Modifier.height(2.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Label("To")
                    t.ownIndex?.let { Spacer(Modifier.width(8.dp)); Pill("Your account #$it", Ink.safe, Icons.shield) }
                }
                Address(t.to, style = Type.mono.copy(fontSize = 16.sp, lineHeight = 25.sp))
                t.note?.let { Text(it, style = Type.small) }
                t.fee?.let { Text("Network fee $it", style = Type.small) }
            }
        }
        Card {
            Row {
                Text("Network fee, at most", style = Type.dim, modifier = Modifier.weight(1f))
                Text(r.fee, style = Type.body.copy(fontWeight = FontWeight.SemiBold))
            }
        }
        for (n in r.notices) Notice(n, Ink.dim, Icons.open)
        Card(Modifier.clickable { details = !details }) {
            Row {
                Text("Details", style = Type.body.copy(fontWeight = FontWeight.SemiBold), modifier = Modifier.weight(1f))
                Text(if (details) "Hide" else "Show", style = Type.small.copy(color = Ink.lamp))
            }
            if (details) {
                for (d in r.details) {
                    Row {
                        Text(d.label, style = Type.small, modifier = Modifier.weight(1f))
                        Text(d.value, style = Type.small.copy(color = Ink.text))
                    }
                }
                Text("From", style = Type.small)
                Address(r.fromAddress, style = Type.mono.copy(fontSize = 13.sp))
            }
        }
    }
}
