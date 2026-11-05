//! The hotkey that opens the palette.

use std::fmt;
use std::str::FromStr;

/// A hotkey error from [`Hotkey::parse`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyError {
    /// The spec is empty.
    #[error("the hotkey is empty")]
    Empty,
    /// The spec has no key after the modifiers.
    #[error("the hotkey `{0}` has no key")]
    MissingKey(String),
    /// A part before the key is not a modifier.
    #[error("`{0}` is not a modifier: use mod, ctrl, meta, alt or shift")]
    UnknownModifier(String),
    /// A modifier is in the spec two times.
    #[error("the modifier `{0}` is in the hotkey two times")]
    RepeatedModifier(String),
    /// The key is not one visible ASCII character or `F1`…`F12`.
    #[error("`{0}` is not a key: use one visible ASCII character or F1 to F12")]
    BadKey(String),
}

/// A key and its modifiers, from a spec such as `mod+k`.
///
/// `mod` is Ctrl or ⌘ (Meta). Both work on all platforms. The other
/// modifiers are `ctrl`, `meta`, `alt` and `shift`. The key is one visible
/// ASCII character or `F1`…`F12`. Case does not matter.
///
/// ```
/// use autumn_command_palette_plugin::Hotkey;
///
/// let hotkey = Hotkey::parse("mod+k").unwrap();
/// assert_eq!(hotkey.aria_keyshortcuts(), "Control+K Meta+K");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one flag for each modifier key"
)]
pub struct Hotkey {
    primary: bool,
    ctrl: bool,
    meta: bool,
    alt: bool,
    shift: bool,
    key: String,
}

impl Hotkey {
    /// Parse a spec such as `mod+k`, `ctrl+shift+p` or `/`.
    ///
    /// # Errors
    ///
    /// [`HotkeyError`] when the spec is empty, has no key, has an unknown
    /// or a repeated modifier, or has a bad key.
    pub fn parse(spec: &str) -> Result<Self, HotkeyError> {
        let trimmed = spec.trim();
        if trimmed.is_empty() {
            return Err(HotkeyError::Empty);
        }
        let parts: Vec<String> = trimmed
            .split('+')
            .map(|p| p.trim().to_lowercase())
            .collect();
        let (key, modifiers) = match parts.split_last() {
            Some((key, modifiers)) if !key.is_empty() => (key, modifiers),
            _ => return Err(HotkeyError::MissingKey(spec.to_owned())),
        };
        let mut hotkey = Self {
            primary: false,
            ctrl: false,
            meta: false,
            alt: false,
            shift: false,
            key: String::new(),
        };
        for modifier in modifiers {
            let flag = match modifier.as_str() {
                "mod" => &mut hotkey.primary,
                "ctrl" => &mut hotkey.ctrl,
                "meta" => &mut hotkey.meta,
                "alt" => &mut hotkey.alt,
                "shift" => &mut hotkey.shift,
                "" => return Err(HotkeyError::MissingKey(spec.to_owned())),
                other => return Err(HotkeyError::UnknownModifier(other.to_owned())),
            };
            if *flag {
                return Err(HotkeyError::RepeatedModifier(modifier.clone()));
            }
            *flag = true;
        }
        if MODIFIERS.contains(&key.as_str()) {
            return Err(HotkeyError::MissingKey(spec.to_owned()));
        }
        if !valid_key(key) {
            return Err(HotkeyError::BadKey(key.clone()));
        }
        hotkey.key.clone_from(key);
        Ok(hotkey)
    }

    /// The normal spec: lowercase, modifiers in a fixed order.
    #[must_use]
    pub fn spec(&self) -> String {
        let mut parts: Vec<&str> = self.flags().into_iter().map(|(name, _)| name).collect();
        parts.push(&self.key);
        parts.join("+")
    }

    /// The `aria-keyshortcuts` value, for example `Control+K Meta+K`.
    #[must_use]
    pub fn aria_keyshortcuts(&self) -> String {
        let key = self.key_label();
        // `mod` gives two shortcuts: one with Control, one with Meta.
        let primaries: &[&str] = if self.primary {
            &["Control", "Meta"]
        } else {
            &[""]
        };
        primaries
            .iter()
            .map(|primary| {
                let mut names: Vec<&str> = Vec::new();
                if self.ctrl || *primary == "Control" {
                    names.push("Control");
                }
                if self.meta || *primary == "Meta" {
                    names.push("Meta");
                }
                if self.alt {
                    names.push("Alt");
                }
                if self.shift {
                    names.push("Shift");
                }
                names.push(&key);
                names.join("+")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The key labels for `kbd`, for example `["Ctrl", "K"]`.
    #[must_use]
    pub fn labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = self
            .flags()
            .into_iter()
            .map(|(_, label)| label.to_owned())
            .collect();
        labels.push(self.key_label());
        labels
    }

    /// The modifiers that are on, in the order of [`spec`](Self::spec):
    /// `(spec name, kbd label)`. `mod` and `ctrl` both show `Ctrl`.
    fn flags(&self) -> Vec<(&'static str, &'static str)> {
        [
            (self.primary, "mod", "Ctrl"),
            (self.ctrl, "ctrl", "Ctrl"),
            (self.meta, "meta", "Meta"),
            (self.alt, "alt", "Alt"),
            (self.shift, "shift", "Shift"),
        ]
        .into_iter()
        .filter(|(on, _, _)| *on)
        .map(|(_, name, label)| (name, label))
        .collect()
    }

    /// The key as people read it: `K`, `F2`, `/`.
    fn key_label(&self) -> String {
        self.key.to_uppercase()
    }

    /// The key, lowercase. The pack JS compares it with `KeyboardEvent.key`.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// `true` when the spec uses `mod`. The label of `mod` is `Ctrl`; the
    /// pack JS shows `⌘` on Apple platforms.
    #[must_use]
    pub const fn uses_primary(&self) -> bool {
        self.primary
    }

    /// `true` when the hotkey has a modifier. A hotkey with no modifier does
    /// nothing while focus is in a text field.
    #[must_use]
    pub const fn has_modifier(&self) -> bool {
        self.primary || self.ctrl || self.meta || self.alt || self.shift
    }
}

/// The modifier names of a spec.
const MODIFIERS: [&str; 5] = ["mod", "ctrl", "meta", "alt", "shift"];

/// `true` for one visible ASCII character (not `+`) or `f1`…`f12`.
fn valid_key(key: &str) -> bool {
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => c.is_ascii_graphic() && c != '+',
        _ => key
            .strip_prefix('f')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=12).contains(&n) && !key.starts_with("f0")),
    }
}

impl Default for Hotkey {
    /// `mod+k`: ⌘K or Ctrl+K.
    fn default() -> Self {
        Self {
            primary: true,
            ctrl: false,
            meta: false,
            alt: false,
            shift: false,
            key: "k".to_owned(),
        }
    }
}

impl FromStr for Hotkey {
    type Err = HotkeyError;

    fn from_str(spec: &str) -> Result<Self, Self::Err> {
        Self::parse(spec)
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.spec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_k_is_the_default() {
        assert_eq!(Hotkey::default(), Hotkey::parse("mod+k").expect("valid"));
        assert_eq!(Hotkey::default().spec(), "mod+k");
    }

    #[test]
    fn parse_normalizes_case_space_and_order() {
        let hotkey = Hotkey::parse(" Shift + CTRL + P ").expect("valid");
        assert_eq!(hotkey.spec(), "ctrl+shift+p");
        assert_eq!(hotkey.to_string(), "ctrl+shift+p");
        assert_eq!("ctrl+shift+p".parse::<Hotkey>(), Ok(hotkey));
    }

    #[test]
    fn aria_keyshortcuts_lists_both_primary_keys() {
        assert_eq!(Hotkey::default().aria_keyshortcuts(), "Control+K Meta+K");
        let hotkey = Hotkey::parse("alt+shift+f2").expect("valid");
        assert_eq!(hotkey.aria_keyshortcuts(), "Alt+Shift+F2");
        assert_eq!(Hotkey::parse("/").expect("valid").aria_keyshortcuts(), "/");
        assert_eq!(
            Hotkey::parse("meta+ctrl+j")
                .expect("valid")
                .aria_keyshortcuts(),
            "Control+Meta+J"
        );
    }

    #[test]
    fn labels_are_kbd_text() {
        assert_eq!(Hotkey::default().labels(), ["Ctrl", "K"]);
        assert_eq!(
            Hotkey::parse("ctrl+alt+shift+meta+x")
                .expect("valid")
                .labels(),
            ["Ctrl", "Meta", "Alt", "Shift", "X"]
        );
        assert_eq!(Hotkey::parse("/").expect("valid").labels(), ["/"]);
    }

    #[test]
    fn a_key_without_a_modifier_has_none() {
        let slash = Hotkey::parse("/").expect("valid");
        assert!(!slash.has_modifier());
        assert!(!slash.uses_primary());
        assert!(Hotkey::default().has_modifier());
        assert!(Hotkey::default().uses_primary());
        assert!(Hotkey::parse("shift+?").expect("valid").has_modifier());
    }

    #[test]
    fn function_keys_parse() {
        assert_eq!(Hotkey::parse("F1").expect("valid").spec(), "f1");
        assert_eq!(Hotkey::parse("f12").expect("valid").labels(), ["F12"]);
        assert_eq!(Hotkey::parse("f13"), Err(HotkeyError::BadKey("f13".into())));
        assert_eq!(Hotkey::parse("f0"), Err(HotkeyError::BadKey("f0".into())));
    }

    #[test]
    fn bad_specs_fail() {
        assert_eq!(Hotkey::parse(""), Err(HotkeyError::Empty));
        assert_eq!(Hotkey::parse("   "), Err(HotkeyError::Empty));
        assert_eq!(
            Hotkey::parse("mod+"),
            Err(HotkeyError::MissingKey("mod+".into()))
        );
        assert_eq!(
            Hotkey::parse("mod"),
            Err(HotkeyError::MissingKey("mod".into()))
        );
        assert_eq!(
            Hotkey::parse("hyper+k"),
            Err(HotkeyError::UnknownModifier("hyper".into()))
        );
        assert_eq!(
            Hotkey::parse("ctrl+ctrl+k"),
            Err(HotkeyError::RepeatedModifier("ctrl".into()))
        );
        assert_eq!(
            Hotkey::parse("mod+enter"),
            Err(HotkeyError::BadKey("enter".into()))
        );
        assert_eq!(Hotkey::parse("mod+é"), Err(HotkeyError::BadKey("é".into())));
    }

    #[test]
    fn plus_is_not_a_key() {
        assert_eq!(
            Hotkey::parse("mod++"),
            Err(HotkeyError::MissingKey("mod++".into()))
        );
    }

    #[test]
    fn errors_have_a_message() {
        assert_eq!(
            HotkeyError::UnknownModifier("x".into()).to_string(),
            "`x` is not a modifier: use mod, ctrl, meta, alt or shift"
        );
    }

    proptest::proptest! {
        /// The normal spec parses to the same hotkey.
        #[test]
        fn spec_round_trips(
            primary in proptest::bool::ANY,
            ctrl in proptest::bool::ANY,
            alt in proptest::bool::ANY,
            shift in proptest::bool::ANY,
            key in "[a-z0-9/.,;?!]",
        ) {
            let mut parts = Vec::new();
            if shift { parts.push("shift"); }
            if alt { parts.push("alt"); }
            if ctrl { parts.push("ctrl"); }
            if primary { parts.push("mod"); }
            parts.push(&key);
            let hotkey = Hotkey::parse(&parts.join("+")).expect("valid");
            proptest::prop_assert_eq!(Hotkey::parse(&hotkey.spec()), Ok(hotkey));
        }

        /// Parse never panics.
        #[test]
        fn parse_does_not_panic(spec in "\\PC{0,20}") {
            let _ = Hotkey::parse(&spec);
        }
    }
}
