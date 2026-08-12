//! `{{var}}` substitution.
//!
//! Deliberately not a template language. No expressions, no filters, no nesting
//! — a translator writing a locale file must never be able to change behavior,
//! only words. An unknown placeholder is left in place rather than blanked, so
//! a missing variable shows up as `{{name}}` on screen where someone will
//! report it, instead of as a sentence with a hole in it.

use super::{Options, Var};

pub(crate) fn interpolate(template: &str, options: &Options<'_>) -> String {
    if !template.contains("{{") {
        return template.to_owned();
    }

    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            // An unclosed placeholder is literal text, not an error: the rest
            // of the string still has to render.
            out.push_str(&rest[start..]);
            return out;
        };
        let name = after[..end].trim();
        match value_of(name, options) {
            Some(value) => out.push_str(&value),
            None => {
                out.push_str("{{");
                out.push_str(&after[..end]);
                out.push_str("}}");
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn value_of(name: &str, options: &Options<'_>) -> Option<String> {
    if let Some(value) = options
        .vars
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
    {
        return Some(match value {
            Var::Str(text) => (*text).to_owned(),
            Var::Num(number) => format_number(*number),
        });
    }
    // `count` is interpolatable without being restated as a var — the shape
    // every i18next call site already assumes.
    if name == "count" {
        return options.count.map(format_number);
    }
    None
}

/// Integers render without a trailing `.0`; everything else through `Display`.
fn format_number(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        format!("{number}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_and_preserves_surrounding_text() {
        let options = Options {
            vars: &[("name", Var::Str("Ann"))],
            ..Options::default()
        };
        assert_eq!(interpolate("Hi {{name}}!", &options), "Hi Ann!");
        assert_eq!(interpolate("no vars here", &options), "no vars here");
    }

    #[test]
    fn an_unknown_placeholder_stays_visible() {
        assert_eq!(
            interpolate("Hi {{name}}", &Options::default()),
            "Hi {{name}}",
            "a blank would hide the bug"
        );
    }

    #[test]
    fn count_interpolates_without_being_restated() {
        let options = Options {
            count: Some(3.0),
            ..Options::default()
        };
        assert_eq!(interpolate("{{count}} items", &options), "3 items");
    }

    #[test]
    fn integers_do_not_render_a_decimal_tail() {
        let options = Options {
            vars: &[("n", Var::Num(2.0)), ("half", Var::Num(2.5))],
            ..Options::default()
        };
        assert_eq!(interpolate("{{n}} / {{half}}", &options), "2 / 2.5");
    }

    #[test]
    fn an_unclosed_placeholder_is_literal_text() {
        assert_eq!(interpolate("a {{b", &Options::default()), "a {{b");
    }
}
