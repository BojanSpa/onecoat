use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::Error;
use crate::jsonc::{Key, Splice};
use crate::model::color::HexColor;
use crate::model::ids::Slot;
use crate::model::palette::Base16Entry;
use crate::model::theme::{OverrideValue, Theme, Validated};

pub const THEME_KEY: &str = "theme";

const DARK_KEY: &str = "dark";
const LIGHT_KEY: &str = "light";
const CHILD_INDENT: usize = 2;
const TOKEN_HINT: &str = "the token names the harness defines, such as `mdHeading`";

const ANSI_16: [(u8, u8, u8); 16] = [
    (0x00, 0x00, 0x00),
    (0xcd, 0x00, 0x00),
    (0x00, 0xcd, 0x00),
    (0xcd, 0xcd, 0x00),
    (0x00, 0x00, 0xee),
    (0xcd, 0x00, 0xcd),
    (0x00, 0xcd, 0xcd),
    (0xe5, 0xe5, 0xe5),
    (0x7f, 0x7f, 0x7f),
    (0xff, 0x00, 0x00),
    (0x00, 0xff, 0x00),
    (0xff, 0xff, 0x00),
    (0x5c, 0x5c, 0xff),
    (0xff, 0x00, 0xff),
    (0x00, 0xff, 0xff),
    (0xff, 0xff, 0xff),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OmpToken {
    Accent,
    BashMode,
    Border,
    BorderAccent,
    BorderMuted,
    CustomMessageBg,
    CustomMessageLabel,
    CustomMessageText,
    Dim,
    Error,
    Link,
    MdCode,
    MdCodeBlock,
    MdCodeBlockBorder,
    MdHeading,
    MdHr,
    MdLink,
    MdLinkUrl,
    MdListBullet,
    MdQuote,
    MdQuoteBorder,
    Muted,
    PythonMode,
    SelectedBg,
    StatusLineBg,
    StatusLineContext,
    StatusLineCost,
    StatusLineDirty,
    StatusLineGitClean,
    StatusLineGitDirty,
    StatusLineModel,
    StatusLineOutput,
    StatusLinePath,
    StatusLineSep,
    StatusLineSpend,
    StatusLineStaged,
    StatusLineSubagents,
    StatusLineUntracked,
    Success,
    SyntaxComment,
    SyntaxFunction,
    SyntaxKeyword,
    SyntaxNumber,
    SyntaxOperator,
    SyntaxPunctuation,
    SyntaxString,
    SyntaxType,
    SyntaxVariable,
    Text,
    ThinkingHigh,
    ThinkingLow,
    ThinkingMax,
    ThinkingMedium,
    ThinkingMinimal,
    ThinkingOff,
    ThinkingText,
    ThinkingXhigh,
    ToolDiffAdded,
    ToolDiffContext,
    ToolDiffRemoved,
    ToolErrorBg,
    ToolOutput,
    ToolPendingBg,
    ToolSuccessBg,
    ToolText,
    ToolTitle,
    UserMessageBg,
    UserMessageText,
    Warning,
}

impl OmpToken {
    pub const ALL: [Self; 69] = [
        Self::Accent,
        Self::BashMode,
        Self::Border,
        Self::BorderAccent,
        Self::BorderMuted,
        Self::CustomMessageBg,
        Self::CustomMessageLabel,
        Self::CustomMessageText,
        Self::Dim,
        Self::Error,
        Self::Link,
        Self::MdCode,
        Self::MdCodeBlock,
        Self::MdCodeBlockBorder,
        Self::MdHeading,
        Self::MdHr,
        Self::MdLink,
        Self::MdLinkUrl,
        Self::MdListBullet,
        Self::MdQuote,
        Self::MdQuoteBorder,
        Self::Muted,
        Self::PythonMode,
        Self::SelectedBg,
        Self::StatusLineBg,
        Self::StatusLineContext,
        Self::StatusLineCost,
        Self::StatusLineDirty,
        Self::StatusLineGitClean,
        Self::StatusLineGitDirty,
        Self::StatusLineModel,
        Self::StatusLineOutput,
        Self::StatusLinePath,
        Self::StatusLineSep,
        Self::StatusLineSpend,
        Self::StatusLineStaged,
        Self::StatusLineSubagents,
        Self::StatusLineUntracked,
        Self::Success,
        Self::SyntaxComment,
        Self::SyntaxFunction,
        Self::SyntaxKeyword,
        Self::SyntaxNumber,
        Self::SyntaxOperator,
        Self::SyntaxPunctuation,
        Self::SyntaxString,
        Self::SyntaxType,
        Self::SyntaxVariable,
        Self::Text,
        Self::ThinkingHigh,
        Self::ThinkingLow,
        Self::ThinkingMax,
        Self::ThinkingMedium,
        Self::ThinkingMinimal,
        Self::ThinkingOff,
        Self::ThinkingText,
        Self::ThinkingXhigh,
        Self::ToolDiffAdded,
        Self::ToolDiffContext,
        Self::ToolDiffRemoved,
        Self::ToolErrorBg,
        Self::ToolOutput,
        Self::ToolPendingBg,
        Self::ToolSuccessBg,
        Self::ToolText,
        Self::ToolTitle,
        Self::UserMessageBg,
        Self::UserMessageText,
        Self::Warning,
    ];

    pub fn name(self) -> &'static str {
        self.spec().0
    }

    pub fn from_name(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|token| token.name() == raw)
    }

    fn spec(self) -> (&'static str, Base16Entry) {
        match self {
            Self::Accent => ("accent", Base16Entry::B0D),
            Self::BashMode => ("bashMode", Base16Entry::B0C),
            Self::Border => ("border", Base16Entry::B02),
            Self::BorderAccent => ("borderAccent", Base16Entry::B0D),
            Self::BorderMuted => ("borderMuted", Base16Entry::B01),
            Self::CustomMessageBg => ("customMessageBg", Base16Entry::B02),
            Self::CustomMessageLabel => ("customMessageLabel", Base16Entry::B0D),
            Self::CustomMessageText => ("customMessageText", Base16Entry::B05),
            Self::Dim => ("dim", Base16Entry::B03),
            Self::Error => ("error", Base16Entry::B08),
            Self::Link => ("link", Base16Entry::B0D),
            Self::MdCode => ("mdCode", Base16Entry::B0B),
            Self::MdCodeBlock => ("mdCodeBlock", Base16Entry::B05),
            Self::MdCodeBlockBorder => ("mdCodeBlockBorder", Base16Entry::B01),
            Self::MdHeading => ("mdHeading", Base16Entry::B0D),
            Self::MdHr => ("mdHr", Base16Entry::B03),
            Self::MdLink => ("mdLink", Base16Entry::B0D),
            Self::MdLinkUrl => ("mdLinkUrl", Base16Entry::B0C),
            Self::MdListBullet => ("mdListBullet", Base16Entry::B0D),
            Self::MdQuote => ("mdQuote", Base16Entry::B0C),
            Self::MdQuoteBorder => ("mdQuoteBorder", Base16Entry::B03),
            Self::Muted => ("muted", Base16Entry::B04),
            Self::PythonMode => ("pythonMode", Base16Entry::B0D),
            Self::SelectedBg => ("selectedBg", Base16Entry::B02),
            Self::StatusLineBg => ("statusLineBg", Base16Entry::B02),
            Self::StatusLineContext => ("statusLineContext", Base16Entry::B0C),
            Self::StatusLineCost => ("statusLineCost", Base16Entry::B0E),
            Self::StatusLineDirty => ("statusLineDirty", Base16Entry::B0A),
            Self::StatusLineGitClean => ("statusLineGitClean", Base16Entry::B0B),
            Self::StatusLineGitDirty => ("statusLineGitDirty", Base16Entry::B0A),
            Self::StatusLineModel => ("statusLineModel", Base16Entry::B0D),
            Self::StatusLineOutput => ("statusLineOutput", Base16Entry::B04),
            Self::StatusLinePath => ("statusLinePath", Base16Entry::B0C),
            Self::StatusLineSep => ("statusLineSep", Base16Entry::B01),
            Self::StatusLineSpend => ("statusLineSpend", Base16Entry::B05),
            Self::StatusLineStaged => ("statusLineStaged", Base16Entry::B0B),
            Self::StatusLineSubagents => ("statusLineSubagents", Base16Entry::B0E),
            Self::StatusLineUntracked => ("statusLineUntracked", Base16Entry::B08),
            Self::Success => ("success", Base16Entry::B0B),
            Self::SyntaxComment => ("syntaxComment", Base16Entry::B03),
            Self::SyntaxFunction => ("syntaxFunction", Base16Entry::B0D),
            Self::SyntaxKeyword => ("syntaxKeyword", Base16Entry::B0E),
            Self::SyntaxNumber => ("syntaxNumber", Base16Entry::B09),
            Self::SyntaxOperator => ("syntaxOperator", Base16Entry::B0C),
            Self::SyntaxPunctuation => ("syntaxPunctuation", Base16Entry::B05),
            Self::SyntaxString => ("syntaxString", Base16Entry::B0B),
            Self::SyntaxType => ("syntaxType", Base16Entry::B0A),
            Self::SyntaxVariable => ("syntaxVariable", Base16Entry::B08),
            Self::Text => ("text", Base16Entry::B05),
            Self::ThinkingHigh => ("thinkingHigh", Base16Entry::B0C),
            Self::ThinkingLow => ("thinkingLow", Base16Entry::B0D),
            Self::ThinkingMax => ("thinkingMax", Base16Entry::B08),
            Self::ThinkingMedium => ("thinkingMedium", Base16Entry::B0C),
            Self::ThinkingMinimal => ("thinkingMinimal", Base16Entry::B04),
            Self::ThinkingOff => ("thinkingOff", Base16Entry::B03),
            Self::ThinkingText => ("thinkingText", Base16Entry::B04),
            Self::ThinkingXhigh => ("thinkingXhigh", Base16Entry::B0A),
            Self::ToolDiffAdded => ("toolDiffAdded", Base16Entry::B0B),
            Self::ToolDiffContext => ("toolDiffContext", Base16Entry::B03),
            Self::ToolDiffRemoved => ("toolDiffRemoved", Base16Entry::B08),
            Self::ToolErrorBg => ("toolErrorBg", Base16Entry::B01),
            Self::ToolOutput => ("toolOutput", Base16Entry::B04),
            Self::ToolPendingBg => ("toolPendingBg", Base16Entry::B00),
            Self::ToolSuccessBg => ("toolSuccessBg", Base16Entry::B01),
            Self::ToolText => ("toolText", Base16Entry::B05),
            Self::ToolTitle => ("toolTitle", Base16Entry::B06),
            Self::UserMessageBg => ("userMessageBg", Base16Entry::B01),
            Self::UserMessageText => ("userMessageText", Base16Entry::B06),
            Self::Warning => ("warning", Base16Entry::B0A),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConfigEdit {
    pub side: Slot,
    pub name: String,
}

#[derive(Serialize)]
struct Document {
    name: String,
    colors: BTreeMap<String, String>,
}

pub fn file_name(slot: Slot) -> String {
    format!("{}.json", crate::render::onecoat_name(slot))
}

pub fn config_edits(slot: Slot) -> Vec<ConfigEdit> {
    vec![ConfigEdit {
        side: slot,
        name: crate::render::onecoat_name(slot),
    }]
}

pub fn theme_file(theme: &Theme<Validated>, slot: Slot) -> Result<String, Error> {
    reject_unknown_tokens(theme)?;

    let colors = OmpToken::ALL
        .into_iter()
        .map(|token| Ok((token.name().to_owned(), color(theme, token)?.to_string())))
        .collect::<Result<BTreeMap<String, String>, Error>>()?;

    let document = Document {
        name: crate::render::onecoat_name(slot),
        colors,
    };

    let mut text = serde_json::to_string_pretty(&document)
        .map_err(|source| Error::JsonEncodeFailed { source })?;

    text.push('\n');

    Ok(text)
}

pub fn config_value(path: &Path, source: &str, side: Slot) -> Result<Option<String>, Error> {
    let lines: Vec<String> = source.split_inclusive('\n').map(str::to_owned).collect();

    let Some(header) = theme_header(&lines) else {
        return Ok(None);
    };

    check_mapping(path, &lines[header])?;

    Ok(child_line(&lines, header, key_of(side)).and_then(|at| current_value(&lines[at])))
}

pub fn splice_config(path: &Path, source: &str, edits: &[ConfigEdit]) -> Result<Splice, Error> {
    let eol = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let mut lines: Vec<String> = source.split_inclusive('\n').map(str::to_owned).collect();
    let mut changed = Vec::new();
    for edit in edits {
        let key = key_of(edit.side);
        let dotted = Key::parse(&format!("{THEME_KEY}.{key}"));

        let Some(header) = theme_header(&lines) else {
            close_last_line(&mut lines, eol);
            lines.push(format!("{THEME_KEY}:{eol}"));
            lines.push(format!(
                "{}{key}: {}{eol}",
                " ".repeat(CHILD_INDENT),
                edit.name
            ));
            changed.push(dotted);
            continue;
        };

        check_mapping(path, &lines[header])?;

        match child_line(&lines, header, key) {
            Some(at) => {
                if current_value(&lines[at]).as_deref() == Some(edit.name.as_str()) {
                    continue;
                }

                lines[at] = replace_value(&lines[at], &edit.name);
                changed.push(dotted);
            }
            None => {
                let at = insert_line(&lines, header);
                open_line(&mut lines[at - 1], eol);
                let indent = child_indent(&lines, header).unwrap_or(CHILD_INDENT);
                lines.insert(
                    at,
                    format!("{}{key}: {}{eol}", " ".repeat(indent), edit.name),
                );
                changed.push(dotted);
            }
        }
    }

    Ok(Splice {
        text: lines.concat(),
        changed,
    })
}

fn reject_unknown_tokens(theme: &Theme<Validated>) -> Result<(), Error> {
    match theme
        .data
        .omp
        .keys()
        .find(|key| OmpToken::from_name(key).is_none())
    {
        Some(key) => Err(Error::UnknownKey {
            path: theme.path.clone(),
            section: "[targets.omp]",
            key: key.clone(),
            accepted: TOKEN_HINT.to_owned(),
        }),
        None => Ok(()),
    }
}

fn color(theme: &Theme<Validated>, token: OmpToken) -> Result<HexColor, Error> {
    match theme.data.omp.get(token.name()) {
        Some(OverrideValue::Color(color)) => Ok(*color),
        Some(OverrideValue::Index(index)) => Ok(xterm_color(*index)),
        Some(OverrideValue::Text(text)) => Err(Error::MalformedColor {
            path: theme.path.clone(),
            key: format!("targets.omp.{}", token.name()),
            value: text.clone(),
        }),
        None => Ok(theme.data.palette[token.spec().1]),
    }
}

fn xterm_color(index: u8) -> HexColor {
    let (r, g, b) = match index {
        0..=15 => ANSI_16[usize::from(index)],
        16..=231 => {
            let level = |value: u8| if value == 0 { 0 } else { 55 + 40 * value };
            let offset = index - 16;
            (
                level(offset / 36),
                level((offset % 36) / 6),
                level(offset % 6),
            )
        }
        _ => {
            let gray = 8 + 10 * (index - 232);
            (gray, gray, gray)
        }
    };

    HexColor::from_rgb(r, g, b)
}

fn key_of(side: Slot) -> &'static str {
    match side {
        Slot::Dark => DARK_KEY,
        Slot::Light => LIGHT_KEY,
    }
}

fn line_body(line: &str) -> &str {
    line.trim_end_matches(['\n', '\r'])
}

fn indent_len(body: &str) -> usize {
    body.len() - body.trim_start_matches([' ', '\t']).len()
}

fn theme_header(lines: &[String]) -> Option<usize> {
    lines.iter().position(|line| {
        line_body(line)
            .strip_prefix(THEME_KEY)
            .is_some_and(|rest| rest.starts_with(':'))
    })
}

fn check_mapping(path: &Path, header: &str) -> Result<(), Error> {
    let body = line_body(header);
    let value = scalar_span(&body[THEME_KEY.len() + 1..]).trim();
    if value.is_empty() {
        return Ok(());
    }

    Err(Error::FieldType {
        path: path.to_path_buf(),
        key: THEME_KEY,
        expected: "a mapping holding dark and light",
        found: if value.starts_with(['{', '[']) {
            "a flow collection"
        } else {
            "a scalar"
        },
    })
}

fn child_indent(lines: &[String], header: usize) -> Option<usize> {
    block_range(lines, header).into_iter().find_map(|line| {
        let body = line_body(&lines[line]);

        (indent_len(body) > 0 && !body.trim_start().starts_with('#')).then(|| indent_len(body))
    })
}

fn child_line(lines: &[String], header: usize, key: &str) -> Option<usize> {
    let indent = child_indent(lines, header);

    block_range(lines, header).into_iter().find(|line| {
        let body = line_body(&lines[*line]);

        Some(indent_len(body)) == indent && key_of_line(body) == Some(key)
    })
}

fn insert_line(lines: &[String], header: usize) -> usize {
    block_range(lines, header)
        .into_iter()
        .rev()
        .find(|line| indent_len(line_body(&lines[*line])) > 0)
        .map_or(header + 1, |line| line + 1)
}

fn block_range(lines: &[String], header: usize) -> std::ops::Range<usize> {
    let end = lines[header + 1..]
        .iter()
        .take_while(|line| in_block(line))
        .count();

    header + 1..header + 1 + end
}

fn in_block(line: &str) -> bool {
    let body = line_body(line);
    let trimmed = body.trim_start();

    body.trim().is_empty() || trimmed.starts_with('#') || indent_len(body) > 0
}

fn key_of_line(body: &str) -> Option<&str> {
    let (key, rest) = body.trim_start().split_once(':')?;

    (rest.is_empty() || rest.starts_with([' ', '\t'])).then_some(key)
}

fn current_value(line: &str) -> Option<String> {
    let value = scalar_span(line_body(line).split_once(':')?.1).trim();
    let bytes = value.as_bytes();

    let unquoted = match (bytes.first().copied(), bytes.last().copied()) {
        (Some(open @ (b'"' | b'\'')), Some(close)) if open == close && bytes.len() >= 2 => {
            &value[1..value.len() - 1]
        }
        _ => value,
    };

    Some(unquoted.to_owned())
}

fn replace_value(line: &str, name: &str) -> String {
    let body = line_body(line);

    let Some(colon) = body.find(':') else {
        return line.to_owned();
    };

    let after = &body[colon + 1..];
    let leading = after.len() - after.trim_start().len();
    let end = colon + 1 + scalar_span(after).trim_end().len();

    let mut replaced = String::with_capacity(line.len() + name.len());
    replaced.push_str(&body[..colon + 1 + leading]);
    replaced.push_str(name);
    replaced.push_str(&body[end..]);
    replaced.push_str(&line[body.len()..]);

    replaced
}

fn scalar_span(text: &str) -> &str {
    let mut quote: Option<u8> = None;
    for (index, byte) in text.bytes().enumerate() {
        if let Some(open) = quote {
            if byte == open {
                quote = None;
            }

            continue;
        }

        match byte {
            b'"' | b'\'' => quote = Some(byte),
            b'#' if index == 0 || text.as_bytes()[index - 1] == b' ' => return &text[..index],
            _ => {}
        }
    }

    text
}

fn open_line(line: &mut String, eol: &str) {
    if !line.ends_with('\n') {
        line.push_str(eol);
    }
}

fn close_last_line(lines: &mut [String], eol: &str) {
    if let Some(last) = lines.last_mut() {
        open_line(last, eol);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/render/omp.rs"]
mod tests;
