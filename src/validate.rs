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
#[path = "../tests/unit/validate.rs"]
mod tests;
