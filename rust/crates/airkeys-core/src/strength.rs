//! Secret-strength classification — a pure kernel, shared by all four surfaces.
//!
//! The point of putting it here rather than in each UI is that the number the
//! user sees must not depend on which client they opened. A password rated
//! "strong" on the desktop and "weak" on the phone is worse than no rating at
//! all: it teaches the user that the rating means nothing.
//!
//! The estimate is a *floor*, not a claim about any particular attacker: the
//! search space implied by the character classes actually used, minus the
//! penalties for the structures that make that space a lie (repeats, runs).

use serde::{Deserialize, Serialize};

/// Bit thresholds between bands. Chosen to line up with the bands the UI paints
/// in `theme.rs`; changing one without the other is how a "green" secret ends up
/// labelled Weak.
const BITS_FAIR: f64 = 40.0;
const BITS_STRONG: f64 = 60.0;
const BITS_EXCELLENT: f64 = 90.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strength {
    Weak,
    Fair,
    Strong,
    Excellent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StrengthReport {
    pub strength: Strength,
    /// Estimated entropy in bits, rounded to one decimal.
    pub bits: f64,
    /// Semantic hints; the shell owns the words, so translations stay out of
    /// the core (and out of the wasm module).
    pub hints: Vec<StrengthHint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrengthHint {
    TooShort,
    NoUppercase,
    NoDigits,
    NoSymbols,
    RepeatedCharacters,
    SequentialRun,
}

/// Classify `secret`. Total: any input, including empty, yields a report.
pub fn secret_strength(secret: &str) -> StrengthReport {
    let chars: Vec<char> = secret.chars().collect();
    if chars.is_empty() {
        return StrengthReport {
            strength: Strength::Weak,
            bits: 0.0,
            hints: vec![StrengthHint::TooShort],
        };
    }

    let mut pool = 0u32;
    let (mut lower, mut upper, mut digit, mut symbol) = (false, false, false, false);
    for c in &chars {
        if c.is_ascii_lowercase() {
            lower = true;
        } else if c.is_ascii_uppercase() {
            upper = true;
        } else if c.is_ascii_digit() {
            digit = true;
        } else {
            symbol = true;
        }
    }
    if lower {
        pool += 26;
    }
    if upper {
        pool += 26;
    }
    if digit {
        pool += 10;
    }
    if symbol {
        // Deliberately not "every codepoint above ASCII". A passphrase in
        // Chinese draws from a huge alphabet in theory and a small one in
        // practice; crediting the theoretical size would rate a common phrase
        // as unbreakable.
        pool += 33;
    }

    let repeats = has_repeat_run(&chars);
    let sequential = has_sequential_run(&chars);

    let mut bits = (chars.len() as f64) * (pool.max(2) as f64).log2();
    // Structure penalties, applied to the estimate rather than the pool: a run
    // shrinks what an attacker must actually search, it does not shrink the
    // alphabet.
    if repeats {
        bits *= 0.75;
    }
    if sequential {
        bits *= 0.75;
    }
    let bits = (bits * 10.0).round() / 10.0;

    let mut hints = Vec::new();
    if chars.len() < 12 {
        hints.push(StrengthHint::TooShort);
    }
    if !upper {
        hints.push(StrengthHint::NoUppercase);
    }
    if !digit {
        hints.push(StrengthHint::NoDigits);
    }
    if !symbol {
        hints.push(StrengthHint::NoSymbols);
    }
    if repeats {
        hints.push(StrengthHint::RepeatedCharacters);
    }
    if sequential {
        hints.push(StrengthHint::SequentialRun);
    }

    let strength = if bits >= BITS_EXCELLENT {
        Strength::Excellent
    } else if bits >= BITS_STRONG {
        Strength::Strong
    } else if bits >= BITS_FAIR {
        Strength::Fair
    } else {
        Strength::Weak
    };

    StrengthReport {
        strength,
        bits,
        hints,
    }
}

/// Three or more of the same character in a row (`aaa`).
fn has_repeat_run(chars: &[char]) -> bool {
    chars.windows(3).any(|w| w[0] == w[1] && w[1] == w[2])
}

/// Three or more consecutive codepoints in a row, either direction (`abc`,
/// `321`).
fn has_sequential_run(chars: &[char]) -> bool {
    chars.windows(3).any(|w| {
        let (a, b, c) = (w[0] as i32, w[1] as i32, w[2] as i32);
        (b - a == 1 && c - b == 1) || (a - b == 1 && b - c == 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_weak_and_says_why() {
        let report = secret_strength("");
        assert_eq!(report.strength, Strength::Weak);
        assert_eq!(report.bits, 0.0);
        assert_eq!(report.hints, vec![StrengthHint::TooShort]);
    }

    #[test]
    fn bands_are_ordered() {
        let weak = secret_strength("hunter2");
        let strong = secret_strength("Tr0ub4dor&3xK9!m");
        assert_eq!(weak.strength, Strength::Weak);
        assert!(strong.bits > weak.bits);
        assert!(matches!(
            strong.strength,
            Strength::Strong | Strength::Excellent
        ));
    }

    #[test]
    fn structure_penalties_apply() {
        // Same length and same character classes; only the structure differs.
        let plain = secret_strength("qxzmvpwtrkbn");
        let repeat = secret_strength("qxzmvpwtrkaaa");
        let run = secret_strength("qxzmvpwtrabc");
        assert!(repeat.bits < plain.bits, "a repeat run must cost bits");
        assert!(run.bits < plain.bits, "a sequential run must cost bits");
        assert!(repeat.hints.contains(&StrengthHint::RepeatedCharacters));
        assert!(run.hints.contains(&StrengthHint::SequentialRun));
    }

    #[test]
    fn hints_are_semantic_not_prose() {
        // The core never emits words — the shell translates these.
        let report = secret_strength("alllowercase");
        assert!(report.hints.contains(&StrengthHint::NoUppercase));
        assert!(report.hints.contains(&StrengthHint::NoDigits));
    }
}
