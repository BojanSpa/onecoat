use std::collections::BTreeMap;
use std::path::Path;

use toml::Value;

use crate::error::{Error, accepted_keys};
use crate::model::color::HexColor;
use crate::model::ids::{Appearance, ThemeId};
use crate::model::palette::{AnsiSet, AnsiSlot, Base16Entry, Palette};
use crate::model::reserved::is_omp_builtin;
use crate::model::theme::{
    OverrideValue, Parsed, RawTheme, Theme, ThemeData, Validated, WtOverrides,
};
use crate::rolemap::derived_ansi;

const WT_KEYS: [&str; 4] = [
    "background",
    "foreground",
    "cursorColor",
    "selectionBackground",
];

const TARGET_SECTIONS: [&str; 3] = ["wt", "herdr", "omp"];

const TOKEN_KEY_HINT: &str = "a token name such as `mdHeading` or `panel_bg`";

const UNFILLED_SLOT: HexColor = HexColor::from_rgb(0, 0, 0);

impl Theme<Parsed> {
    #[allow(missing_docs)]
    pub fn validate(self) -> Result<Theme<Validated>, Error> {
        let Theme {
            id,
            name,
            appearance,
            derived,
            origin,
            path,
            data,
        } = self;
        let RawTheme {
            palette: raw_palette,
            ansi: raw_ansi,
            targets,
        } = data;

        let palette = complete_palette(&path, &raw_palette)?;
        check_appearance(&path, appearance, &palette)?;
        check_id_is_free(&path, &id)?;
        let ansi = ansi_with_overrides(&path, &palette, &raw_ansi)?;
        let wt = wt_overrides(&path, targets.get("wt"))?;

        let mut omp = BTreeMap::new();
        let mut herdr = BTreeMap::new();
        for (section, keys) in &targets {
            let (tokens, section_name) = match section.as_str() {
                "herdr" => (&mut herdr, "[targets.herdr]"),
                "omp" => (&mut omp, "[targets.omp]"),
                "wt" => continue,
                other => {
                    return Err(Error::UnknownKey {
                        path,
                        section: "[targets]",
                        key: other.to_owned(),
                        accepted: accepted_keys(&TARGET_SECTIONS),
                    });
                }
            };
            for (key, value) in keys {
                if !is_token_name(key) {
                    return Err(Error::UnknownKey {
                        path: path.clone(),
                        section: section_name,
                        key: key.clone(),
                        accepted: TOKEN_KEY_HINT.to_owned(),
                    });
                }
                tokens.insert(key.clone(), token_value(&path, section, key, value)?);
            }
        }

        Ok(Theme {
            id,
            name,
            appearance,
            derived,
            origin,
            path,
            data: ThemeData {
                palette,
                ansi,
                wt,
                omp,
                herdr,
            },
        })
    }
}

fn complete_palette(path: &Path, raw: &BTreeMap<String, Value>) -> Result<Palette, Error> {
    let mut present = [false; 16];
    let mut slots = [UNFILLED_SLOT; 16];
    let mut first_malformed: Option<(String, String)> = None;
    for (key, value) in raw {
        let entry = Base16Entry::from_name(key).ok_or_else(|| palette_key_rejected(path, key))?;
        if present[entry.index()] {
            return Err(palette_key_rejected(path, key));
        }
        present[entry.index()] = true;
        if let Some(color) = value.as_str().and_then(HexColor::parse) {
            slots[entry.index()] = color;
        } else if first_malformed.is_none() {
            first_malformed = Some((key.clone(), value_text(value)));
        }
    }
    let missing: Vec<Base16Entry> = Base16Entry::ALL
        .into_iter()
        .filter(|entry| !present[entry.index()])
        .collect();
    if !missing.is_empty() {
        return Err(Error::PaletteIncomplete {
            path: path.to_path_buf(),
            missing,
        });
    }
    if let Some((key, value)) = first_malformed {
        return Err(Error::MalformedColor {
            path: path.to_path_buf(),
            key,
            value,
        });
    }
    Ok(Palette::from(slots))
}

fn check_appearance(path: &Path, declared: Appearance, palette: &Palette) -> Result<(), Error> {
    let background = palette[Base16Entry::B00];
    let luminance = background.luminance();
    let expected = if luminance >= 0.5 {
        Appearance::Light
    } else {
        Appearance::Dark
    };
    if expected == declared {
        return Ok(());
    }
    Err(Error::AppearanceMismatch {
        path: path.to_path_buf(),
        background,
        declared,
        luminance,
        expected,
    })
}

fn check_id_is_free(path: &Path, id: &ThemeId) -> Result<(), Error> {
    if is_omp_builtin(id.as_str()) {
        return Err(Error::ReservedId {
            path: path.to_path_buf(),
            id: id.clone(),
        });
    }
    Ok(())
}

fn ansi_with_overrides(
    path: &Path,
    palette: &Palette,
    raw: &BTreeMap<String, Value>,
) -> Result<AnsiSet, Error> {
    let mut ansi = derived_ansi(palette);
    for (key, value) in raw {
        let slot = AnsiSlot::from_name(key).ok_or_else(|| ansi_unknown(path, key))?;
        ansi[slot] = color_value(path, &format!("ansi.{key}"), value)?;
    }
    Ok(ansi)
}

fn wt_overrides(path: &Path, keys: Option<&BTreeMap<String, Value>>) -> Result<WtOverrides, Error> {
    let mut wt = WtOverrides {
        background: None,
        foreground: None,
        cursor_color: None,
        selection_background: None,
    };
    let Some(keys) = keys else {
        return Ok(wt);
    };
    for (key, value) in keys {
        let field = match key.as_str() {
            "background" => &mut wt.background,
            "foreground" => &mut wt.foreground,
            "cursorColor" => &mut wt.cursor_color,
            "selectionBackground" => &mut wt.selection_background,
            other => {
                return Err(Error::UnknownKey {
                    path: path.to_path_buf(),
                    section: "[targets.wt]",
                    key: other.to_owned(),
                    accepted: accepted_keys(&WT_KEYS),
                });
            }
        };
        *field = Some(color_value(path, &format!("targets.wt.{key}"), value)?);
    }
    Ok(wt)
}

fn palette_key_rejected(path: &Path, key: &str) -> Error {
    Error::UnknownKey {
        path: path.to_path_buf(),
        section: "[palette]",
        key: key.to_owned(),
        accepted: accepted_keys(&Base16Entry::ALL.map(Base16Entry::name)),
    }
}

fn ansi_unknown(path: &Path, key: &str) -> Error {
    Error::UnknownKey {
        path: path.to_path_buf(),
        section: "[ansi]",
        key: key.to_owned(),
        accepted: accepted_keys(&AnsiSlot::ALL.map(AnsiSlot::name)),
    }
}

fn color_value(path: &Path, key: &str, value: &Value) -> Result<HexColor, Error> {
    value
        .as_str()
        .and_then(HexColor::parse)
        .ok_or_else(|| Error::MalformedColor {
            path: path.to_path_buf(),
            key: key.to_owned(),
            value: value_text(value),
        })
}

fn token_value(
    path: &Path,
    section: &str,
    key: &str,
    value: &Value,
) -> Result<OverrideValue, Error> {
    let invalid = || Error::OverrideValueInvalid {
        path: path.to_path_buf(),
        key: format!("targets.{section}.{key}"),
        value: value_text(value),
    };
    match value {
        Value::String(text) => match text.strip_prefix('#') {
            Some(_) => HexColor::parse(text)
                .map(OverrideValue::Color)
                .ok_or_else(|| Error::MalformedColor {
                    path: path.to_path_buf(),
                    key: format!("targets.{section}.{key}"),
                    value: text.clone(),
                }),
            None => Ok(OverrideValue::Text(text.clone())),
        },
        Value::Integer(number) => u8::try_from(*number)
            .map(OverrideValue::Index)
            .map_err(|_| invalid()),
        Value::Float(_)
        | Value::Boolean(_)
        | Value::Datetime(_)
        | Value::Array(_)
        | Value::Table(_) => Err(invalid()),
    }
}

fn is_token_name(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {
            chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        }
        _ => false,
    }
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::Error;
    use crate::model::color::HexColor;
    use crate::model::ids::{Appearance, Origin};
    use crate::model::palette::{AnsiSlot, Base16Entry};
    use crate::model::theme::{OverrideValue, Theme, Validated};

    fn validate_like_themeset(name: &str, source: &str) -> Result<Theme<Validated>, Error> {
        let path = PathBuf::from(format!("tests/fixtures/themes/{name}"));
        Theme::parse(path, source, Origin::User)?.validate()
    }

    macro_rules! source {
        ($name:literal) => {
            include_str!(concat!("../tests/fixtures/themes/", $name))
        };
    }

    macro_rules! fixture {
        ($name:literal) => {
            validate_like_themeset($name, source!($name))
        };
    }

    #[test]
    fn incomplete_palette_names_the_missing_entry() {
        let error = fixture!("incomplete-palette.toml").unwrap_err();
        let message = error.to_string();
        assert!(
            matches!(error, Error::PaletteIncomplete { .. }),
            "{message}"
        );
        assert!(
            message.contains("tests/fixtures/themes/incomplete-palette.toml"),
            "{message}"
        );
        assert!(message.contains("base0D"), "{message}");
        assert!(message.contains("[palette]"), "{message}");
    }

    #[test]
    fn malformed_palette_colour_names_the_key_and_value() {
        let error = fixture!("malformed-color.toml").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("base07"), "{message}");
        assert!(message.contains("#xyz"), "{message}");
    }

    #[test]
    fn a_declaration_that_contradicts_the_background_is_rejected() {
        let error = fixture!("light-declared-dark.toml").unwrap_err();
        let message = error.to_string();
        assert!(
            matches!(error, Error::AppearanceMismatch { .. }),
            "{message}"
        );
        assert!(message.contains("appearance = \"light\""), "{message}");
        assert!(message.contains("set appearance = \"dark\""), "{message}");

        let error = fixture!("dark-declared-light.toml").unwrap_err();
        let message = error.to_string();
        assert!(
            matches!(error, Error::AppearanceMismatch { .. }),
            "{message}"
        );
        assert!(message.contains("set appearance = \"light\""), "{message}");
    }

    #[test]
    fn declarations_that_match_the_background_validate() {
        let dark = source!("light-declared-dark.toml")
            .replace("appearance = \"light\"", "appearance = \"dark\"");
        let theme = validate_like_themeset("light-declared-dark.toml", &dark).unwrap();
        assert_eq!(theme.appearance, Appearance::Dark);

        let light = source!("dark-declared-light.toml")
            .replace("appearance = \"dark\"", "appearance = \"light\"");
        let theme = validate_like_themeset("dark-declared-light.toml", &light).unwrap();
        assert_eq!(theme.appearance, Appearance::Light);
    }

    #[test]
    fn reserved_omp_builtin_ids_are_rejected() {
        let error = fixture!("reserved-id.toml").unwrap_err();
        let message = error.to_string();
        assert!(matches!(error, Error::ReservedId { .. }), "{message}");
        assert!(message.contains("titanium"), "{message}");
        assert!(message.contains("rename"), "{message}");
    }

    #[test]
    fn unknown_ansi_key_lists_the_windows_terminal_keys() {
        let error = fixture!("unknown-ansi-key.toml").unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("unknown key `magenta` in [ansi]"),
            "{message}"
        );
        assert!(message.contains("purple"), "{message}");
    }

    #[test]
    fn unknown_root_key_is_rejected() {
        let error = fixture!("unknown-root-key.toml").unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("unknown key `foo` in the theme file"),
            "{message}"
        );
    }

    #[test]
    fn overrides_are_interpreted_into_the_validated_theme() {
        let theme = fixture!("overrides.toml").unwrap();
        let data = &theme.data;
        assert_eq!(
            data.ansi[AnsiSlot::Red],
            HexColor::parse("#ff0000").unwrap()
        );
        assert_eq!(
            data.ansi[AnsiSlot::Green],
            data.palette[Base16Entry::B0B],
            "an ANSI slot the file did not override keeps its derived colour"
        );
        assert_eq!(data.wt.cursor_color, HexColor::parse("#d8dee9"));
        assert_eq!(
            data.omp.get("mdHeading"),
            Some(&OverrideValue::Color(HexColor::parse("#81a1c1").unwrap()))
        );
        assert_eq!(
            data.omp.get("thinkingOff"),
            Some(&OverrideValue::Index(240))
        );
        assert_eq!(
            data.herdr.get("name"),
            Some(&OverrideValue::Text("terminal".to_owned()))
        );
    }

    #[test]
    fn a_second_spelling_of_a_palette_key_is_rejected() {
        let source = source!("overrides.toml").replace(
            "base0A = \"#EBCB8B\"",
            "base0A = \"#EBCB8B\"\nbase0a = \"#000000\"",
        );
        let error = validate_like_themeset("duplicate.toml", &source).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("unknown key `base0a` in [palette]"),
            "{message}"
        );
        assert!(message.contains("base0A"), "{message}");
    }

    #[test]
    fn unknown_target_section_lists_the_targets() {
        let source = source!("overrides.toml").replace("[targets.herdr]", "[targets.vscode]");
        let error = validate_like_themeset("unknown-target.toml", &source).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("unknown key `vscode` in [targets]"),
            "{message}"
        );
        assert!(
            message.contains("accepted keys are wt, herdr, omp"),
            "{message}"
        );
    }

    #[test]
    fn windows_terminal_overrides_reject_ansi_keys() {
        let source = source!("overrides.toml").replace(
            "cursorColor = \"#D8DEE9\"",
            "cursorColor = \"#D8DEE9\"\nred = \"#ff0000\"",
        );
        let error = validate_like_themeset("wt-key.toml", &source).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("unknown key `red` in [targets.wt]"),
            "{message}"
        );
        assert!(
            message.contains("background, foreground, cursorColor, selectionBackground"),
            "{message}"
        );
    }

    #[test]
    fn out_of_range_token_indices_are_rejected() {
        let source = source!("overrides.toml").replace("thinkingOff = 240", "thinkingOff = 300");
        let error = validate_like_themeset("index.toml", &source).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("targets.omp.thinkingOff"), "{message}");
        assert!(message.contains("300"), "{message}");
    }

    #[test]
    fn malformed_token_colours_are_rejected() {
        let source =
            source!("overrides.toml").replace("mdHeading = \"#81A1C1\"", "mdHeading = \"#81A1\"");
        let error = validate_like_themeset("token-color.toml", &source).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("targets.omp.mdHeading"), "{message}");
        assert!(message.contains("#81A1"), "{message}");
    }
}
