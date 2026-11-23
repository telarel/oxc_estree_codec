use std::borrow::Cow;

use oxc::allocator::GetAllocator;
use oxc::str::{JSStr, JSStrBuilder, Str};
use sonic_rs::{Deserializer, Read, Value};

/// The UTF-8 bytes of `U+FFFD`.
const REPLACEMENT_CHARACTER: &[u8] = b"\xEF\xBF\xBD";

fn hex_code_point(bytes: &[u8]) -> Option<u16> {
    if bytes.len() != 4 {
        return None;
    }

    let mut value: u32 = 0;

    for byte in bytes {
        let digit: u32 = match byte {
            | b'0'..=b'9' => u32::from(byte - b'0'),
            | b'a'..=b'f' => u32::from(byte - b'a') + 10,
            | b'A'..=b'F' => u32::from(byte - b'A') + 10,
            | _ => return None,
        };

        value = (value << 4) | digit;
    }

    u16::try_from(value).ok()
}

fn decode_lone_surrogate_encoding<'a>(
    value: &str,
    allocator: &impl GetAllocator<'a>,
) -> Option<JSStr<'a>> {
    if !value.contains('\u{FFFD}') {
        return None;
    }

    let mut builder: JSStrBuilder<'a> =
        JSStrBuilder::with_capacity_in(value.len(), allocator);

    let mut rest: &str = value;

    while let Some(index) = rest.find('\u{FFFD}') {
        builder.push_str(&rest[..index]);

        let after_marker: &str = &rest[index + '\u{FFFD}'.len_utf8()..];

        let hex: &str = after_marker.get(..4)?;

        if !hex.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) {
            return None;
        }

        let code_unit: u16 = hex_code_point(hex.as_bytes())?;

        // `fffd` is the marker for a literal `U+FFFD`. Any other code unit is
        // pushed as itself, which restores lone surrogates and also lets
        // `U+FFFD` + `fffd` decode to `U+FFFD`.
        builder.push_code_unit(code_unit);

        rest = &after_marker[4..];
    }

    builder.push_str(rest);

    Some(builder.into_js_str())
}

/// Rewrite the JSON text into the reader's intermediate encoding before
/// parsing.
///
/// It encodes two kinds of content as `U+FFFD` followed by 4 lowercase hex
/// digits:
///
/// - an unpaired surrogate `\uXXXX` escape, encoded as the 4 lowercase hex
///   digits of its code unit. [`read_js_str`] decodes that marker back into a
///   lone surrogate.
/// - every literal or `\uFFFD`-escaped `U+FFFD`, encoded as `fffd`.
///   [`read_js_str`] decodes that marker back into `U+FFFD`.
///   [`read_source_str`] restores the marker in source-text fields.
fn encode_lone_surrogate_escapes(input: &str) -> Cow<'_, str> {
    if !input.contains('\u{FFFD}') && !input.contains("\\u") {
        return Cow::Borrowed(input);
    }

    let bytes: &[u8] = input.as_bytes();

    let mut needs_rewrite: Option<usize> = None;

    let mut i: usize = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(REPLACEMENT_CHARACTER) {
            if needs_rewrite.is_none() {
                needs_rewrite = Some(i);
            }

            i += REPLACEMENT_CHARACTER.len();
            continue;
        }

        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }

        if i + 6 > bytes.len() {
            i += 2;
            continue;
        }

        match bytes.get(i + 1) {
            | Some(b'u') => {},
            | Some(b'\\') => {
                i += 2;
                continue;
            },
            | _ => {
                i += 2;
                continue;
            },
        }

        let Some(code) = hex_code_point(&bytes[i + 2..i + 6]) else {
            i += 2;
            continue;
        };

        let is_high: bool = (0xD800..0xDC00).contains(&code);

        let is_low: bool = (0xDC00..0xE000).contains(&code);

        if !is_high && !is_low {
            if code == 0xFFFD && needs_rewrite.is_none() {
                needs_rewrite = Some(i);
            }

            i += 6;
            continue;
        }

        let paired: bool = is_high
            && i + 12 <= bytes.len()
            && bytes[i + 6] == b'\\'
            && bytes[i + 7] == b'u'
            && hex_code_point(&bytes[i + 8..i + 12])
                .is_some_and(|low| (0xDC00..0xE000).contains(&low));

        if paired {
            i += 12;
            continue;
        }

        if needs_rewrite.is_none() {
            needs_rewrite = Some(i);
        }

        i += 6;
    }

    let Some(start) = needs_rewrite else {
        return Cow::Borrowed(input);
    };

    let mut encoded: String = String::with_capacity(input.len() + 8);

    encoded.push_str(&input[..start]);

    let mut i: usize = start;

    while i < bytes.len() {
        if bytes[i..].starts_with(REPLACEMENT_CHARACTER) {
            encoded.push('\u{FFFD}');
            encoded.push_str("fffd");

            i += REPLACEMENT_CHARACTER.len();
            continue;
        }

        if bytes[i] == b'\\' {
            match bytes.get(i + 1) {
                // JSON-escaped backslash (`\\`): copy both bytes verbatim so a
                // following `u` (e.g. the `raw` field's `\\ud834`) is never
                // mistaken for a surrogate escape.
                | Some(b'\\') => {
                    encoded.push_str(&input[i..i + 2]);
                    i += 2;
                    continue;
                },
                | Some(b'u')
                    if hex_code_point(
                        &bytes[i + 2..(i + 6).min(bytes.len())],
                    ) == Some(0xFFFD) =>
                {
                    encoded.push('\u{FFFD}');
                    encoded.push_str("fffd");

                    i += 6;
                    continue;
                },
                | Some(b'u')
                    if hex_code_point(
                        &bytes[i + 2..(i + 6).min(bytes.len())],
                    )
                    .is_some_and(|code| (0xD800..0xE000).contains(&code)) =>
                {
                    let code: u16 =
                        hex_code_point(&bytes[i + 2..(i + 6).min(bytes.len())])
                            .unwrap_or_default();

                    let paired: bool = (0xD800..0xDC00).contains(&code)
                        && i + 12 <= bytes.len()
                        && bytes[i + 6] == b'\\'
                        && bytes[i + 7] == b'u'
                        && hex_code_point(&bytes[i + 8..i + 12])
                            .is_some_and(|low| (0xDC00..0xE000).contains(&low));

                    if paired {
                        encoded.push_str(&input[i..i + 12]);
                        i += 12;
                    } else {
                        encoded.push('\u{FFFD}');
                        let code: u32 = u32::from(code);
                        encoded.push_str(&format!("{code:04x}"));
                        i += 6;
                    }
                    continue;
                },
                | _ => {},
            }
        }

        let char_end: usize = {
            let mut end: usize = i + 1;

            while end < bytes.len() && (bytes[end] & 0xC0) == 0x80 {
                end += 1;
            }

            end
        };

        encoded.push_str(&input[i..char_end]);

        i = char_end;
    }

    Cow::Owned(encoded)
}

/// Decode the reader's intermediate encoding into a JavaScript string.
///
/// JSON cannot represent lone surrogates. A literal `U+FFFD` followed by 4
/// lowercase hex digits also looks the same as the encoding. Therefore
/// [`encode_lone_surrogate_escapes`] rewrites an unpaired surrogate escape and
/// every literal or `\uFFFD`-escaped `U+FFFD` as `U+FFFD` followed by 4
/// lowercase hex digits. [`read_js_str`] decodes those markers into a [`JSStr`].
/// The resulting string preserves lone surrogates and literal `U+FFFD` through
/// the ESTree round-trip.
///
/// This is for decoded values (`value`, `cooked`). Source-text fields (`raw`,
/// regex `pattern`, directives, hashbang, JSX text) use [`read_source_str`]
/// instead, which restores only the `fffd` marker.
///
/// A DOM built outside [`parse`] (for example through the doc-hidden
/// `__internal::ProgramReader`) has no markers. In such a DOM, [`read_js_str`]
/// decodes a literal `U+FFFD` followed by 4 lowercase hex digits as a marker.
pub fn read_js_str<'a>(
    value: &str,
    allocator: &impl GetAllocator<'a>,
) -> JSStr<'a> {
    decode_lone_surrogate_encoding(value, allocator)
        .unwrap_or_else(|| JSStr::from_str_in(value, allocator))
}

/// Restore source text from the reader's intermediate encoding.
///
/// [`encode_lone_surrogate_escapes`] encodes every literal or `\uFFFD`-escaped
/// `U+FFFD` as `U+FFFD` followed by `fffd`. This function decodes only that
/// marker. It copies all other bytes verbatim, including `U+FFFD` followed by
/// any other 4 hex digits and the surrogate markers (`d800`-`dfff`).
///
/// A DOM built outside [`parse`] (for example through the doc-hidden
/// `__internal::ProgramReader`) has no markers. In such a DOM,
/// [`read_source_str`] deletes the `fffd` that follows `U+FFFD`.
pub fn read_source_str<'a>(
    value: &str,
    allocator: &impl GetAllocator<'a>,
) -> Str<'a> {
    if !value.contains('\u{FFFD}') {
        return Str::from_str_in(value, allocator);
    }

    let mut restored: String = String::with_capacity(value.len());

    let mut rest: &str = value;

    while let Some(index) = rest.find('\u{FFFD}') {
        restored.push_str(&rest[..index]);
        restored.push('\u{FFFD}');

        let after_marker: &str = &rest[index + '\u{FFFD}'.len_utf8()..];

        rest = after_marker.strip_prefix("fffd").unwrap_or(after_marker);
    }

    restored.push_str(rest);

    Str::from_str_in(&restored, allocator)
}

pub fn parse(input: &str) -> Result<Value, String> {
    let prepared: Cow<'_, str> = encode_lone_surrogate_escapes(input);

    let mut deserializer: Deserializer<Read<'_>> =
        Deserializer::from_str(prepared.as_ref()).use_rawnumber();

    let value: Value = deserializer
        .deserialize::<Value>()
        .map_err(|error| error.to_string())?;

    deserializer.end().map_err(|error| error.to_string())?;

    Ok(value)
}
