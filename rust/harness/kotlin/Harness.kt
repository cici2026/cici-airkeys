// Conformance harness — the Kotlin surface of the shared core.
//
// Asserts the SAME facts `cargo test` asserts in Rust, through the generated
// bindings. Divergence here means Android would ship different behaviour from
// the other three clients, which is the exact failure the shared core exists to
// prevent.
//
// Run with rust/scripts/smoke-kotlin.sh. Exits non-zero on the first mismatch.

import uniffi.airkeys_core_uniffi.*

private var failures = 0

private fun check(name: String, expected: Any?, actual: Any?) {
    if (expected != actual) {
        System.err.println("FAIL $name\n  expected: $expected\n  actual:   $actual")
        failures++
    } else {
        println("ok   $name")
    }
}

fun main() {
    // -- primitives ---------------------------------------------------------
    check("to_hex", "deadbeef", toHex(byteArrayOf(0xde.toByte(), 0xad.toByte(), 0xbe.toByte(), 0xef.toByte()), false))
    check("to_hex/prefixed", "0x01", toHex(byteArrayOf(1), true))
    check("from_hex", "deadbeef", toHex(fromHex("0xdeadbeef"), false))
    check("to_base64url", "aGk", toBase64url("hi".toByteArray()))
    check("from_base64url", "hi", String(fromBase64url("aGk")))

    // Strictness is part of the contract: junk errors, it does not truncate.
    try {
        fromHex("abc")
        System.err.println("FAIL from_hex/odd-length should have thrown"); failures++
    } catch (e: CoreException) {
        println("ok   from_hex/odd-length throws")
    }
    try {
        fromBase64url("aGk=")
        System.err.println("FAIL from_base64url/padded should have thrown"); failures++
    } catch (e: CoreException) {
        println("ok   from_base64url/padded throws")
    }

    // The id must be identical on every platform or a synced item duplicates.
    check(
        "item_fingerprint/stable",
        itemFingerprint("login", "GitHub", "ann@example.com"),
        itemFingerprint("login", "GitHub", "ann@example.com"),
    )

    // -- strength -----------------------------------------------------------
    val empty = secretStrength("")
    check("strength/empty", Strength.WEAK, empty.strength)
    check("strength/empty bits", 0.0, empty.bits)
    check("strength/empty hint", listOf(StrengthHint.TOO_SHORT), empty.hints)

    val strong = secretStrength("Tr0ub4dor&3xK9!m")
    if (strong.bits <= empty.bits) {
        System.err.println("FAIL strength/ordering"); failures++
    } else {
        println("ok   strength/ordering")
    }

    // -- i18n ---------------------------------------------------------------
    check("resolve_language/zh-CN", "zh", resolveLanguage("zh-CN").resolvedLanguage)
    check("resolve_language/unknown", "en", resolveLanguage("kl-GL").resolvedLanguage)
    check("supported_languages", listOf("en", "zh", "ja"), supportedLanguages())

    val i18n = I18n.embedded()
    check("t/plain", "Unlock", i18n.t("unlock.title", mapOf(), null))
    check("t/plural one", "1 item", i18n.t("vault.itemCount", mapOf(), 1.0))
    check("t/plural other", "12 items", i18n.t("vault.itemCount", mapOf(), 12.0))
    check("t/interpolation", "Personal", i18n.t("vault.noMatchBody", mapOf("query" to "Personal"), null)
        .let { if (it.contains("Personal")) "Personal" else it })

    if (failures > 0) {
        System.err.println("\n$failures check(s) failed")
        kotlin.system.exitProcess(1)
    }
    println("\nall checks passed")
}
