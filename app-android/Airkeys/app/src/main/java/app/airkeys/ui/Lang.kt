package app.airkeys.ui

import android.content.res.Configuration
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.compositionLocalOf
import app.airkeys.core.Language

/**
 * The language the core draws in (`view.language`). The shell's own words follow it, not the
 * system, so the shell and the core never disagree on a screen.
 */
val LocalLanguage = compositionLocalOf { Language.EN }

/** `tr(lang, "English", "中文")`: one of the shell's own words, in [lang]. */
fun tr(lang: Language, en: String, zh: String) = if (lang == Language.ZH) zh else en

/** One of the shell's own words, in the language of the screen being drawn. */
@Composable
@ReadOnlyComposable
fun tr(en: String, zh: String) = tr(LocalLanguage.current, en, zh)

/**
 * The app's language: Chinese for any Chinese locale (any region or script), else English.
 * The Activity's configuration carries the per-app language (Android 13+) as well as the system's.
 */
fun languageOf(config: Configuration): Language =
    if (config.locales[0].language == "zh") Language.ZH else Language.EN
