// Conformance harness — the Swift surface of the shared core.
//
// Asserts the SAME facts `cargo test` asserts in Rust, through the generated
// bindings. See harness/kotlin/Harness.kt for why this is a separate claim.
//
// Run with rust/scripts/smoke-swift.sh. Exits non-zero on the first mismatch.

// No `import airkeys_core_uniffi`: this file and the generated Swift are
// compiled into a single module, so the bindings are already in scope.
import Foundation

var failures = 0

func check<T: Equatable>(_ name: String, _ expected: T, _ actual: T) {
    if expected != actual {
        FileHandle.standardError.write(
            "FAIL \(name)\n  expected: \(expected)\n  actual:   \(actual)\n".data(using: .utf8)!)
        failures += 1
    } else {
        print("ok   \(name)")
    }
}

// -- primitives -------------------------------------------------------------
check("to_hex", "deadbeef", toHex(bytes: Data([0xde, 0xad, 0xbe, 0xef]), prefix: false))
check("to_hex/prefixed", "0x01", toHex(bytes: Data([0x01]), prefix: true))
check("from_hex", "deadbeef", toHex(bytes: try! fromHex(text: "0xdeadbeef"), prefix: false))
check("to_base64url", "aGk", toBase64url(bytes: "hi".data(using: .utf8)!))
check("from_base64url", "hi", String(data: try! fromBase64url(text: "aGk"), encoding: .utf8)!)

// Strictness is part of the contract: junk errors, it does not truncate.
do {
    _ = try fromHex(text: "abc")
    FileHandle.standardError.write("FAIL from_hex/odd-length should have thrown\n".data(using: .utf8)!)
    failures += 1
} catch {
    print("ok   from_hex/odd-length throws")
}
do {
    _ = try fromBase64url(text: "aGk=")
    FileHandle.standardError.write("FAIL from_base64url/padded should have thrown\n".data(using: .utf8)!)
    failures += 1
} catch {
    print("ok   from_base64url/padded throws")
}

// The id must be identical on every platform or a synced item duplicates.
check(
    "item_fingerprint/stable",
    itemFingerprint(kind: "login", title: "GitHub", account: "ann@example.com"),
    itemFingerprint(kind: "login", title: "GitHub", account: "ann@example.com"))

// -- strength ---------------------------------------------------------------
let empty = secretStrength(secret: "")
check("strength/empty", Strength.weak, empty.strength)
check("strength/empty bits", 0.0, empty.bits)
check("strength/empty hint", [StrengthHint.tooShort], empty.hints)

let strong = secretStrength(secret: "Tr0ub4dor&3xK9!m")
if strong.bits <= empty.bits {
    FileHandle.standardError.write("FAIL strength/ordering\n".data(using: .utf8)!)
    failures += 1
} else {
    print("ok   strength/ordering")
}

// -- i18n -------------------------------------------------------------------
check("resolve_language/zh-CN", "zh", resolveLanguage(requested: "zh-CN").resolvedLanguage)
check("resolve_language/unknown", "en", resolveLanguage(requested: "kl-GL").resolvedLanguage)
check("supported_languages", ["en", "zh", "ja"], supportedLanguages())

let i18n = try! I18n.embedded()
check("t/plain", "Unlock", try! i18n.t(key: "unlock.title", vars: [:], count: nil))
check("t/plural one", "1 item", try! i18n.t(key: "vault.itemCount", vars: [:], count: 1.0))
check("t/plural other", "12 items", try! i18n.t(key: "vault.itemCount", vars: [:], count: 12.0))
check(
    "t/interpolation", true,
    try! i18n.t(key: "vault.noMatchBody", vars: ["query": "Personal"], count: nil).contains("Personal"))

if failures > 0 {
    FileHandle.standardError.write("\n\(failures) check(s) failed\n".data(using: .utf8)!)
    exit(1)
}
print("\nall checks passed")
