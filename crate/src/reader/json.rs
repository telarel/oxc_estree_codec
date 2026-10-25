use std::borrow::Cow;

use oxc::allocator::GetAllocator;
use oxc::str::{JSStr, JSStrBuilder};
use sonic_rs::Value;

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

        builder.push_code_unit(code_unit);

        rest = &after_marker[4..];
    }

    builder.push_str(rest);

    Some(builder.into_js_str())
}

/// Rewrite JSON `\uXXXX` escapes for unpaired surrogates into the reader's
/// intermediate lone-surrogate encoding (`U+FFFD` followed by 4 lowercase hex
/// chars), which [`read_js_str`] decodes back into a [`JSStr`].
fn encode_lone_surrogate_escapes(input: &str) -> Cow<'_, str> {
    let bytes: &[u8] = input.as_bytes();

    let mut needs_rewrite: Option<usize> = None;

    let mut i: usize = 0;

    while i + 6 <= bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
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

/// Decode the reader's intermediate lone-surrogate encoding into a JavaScript
/// string.
///
/// JSON cannot represent lone surrogates, so [`encode_lone_surrogate_escapes`]
/// rewrites unpaired surrogate escapes as `U+FFFD` followed by 4 lowercase hex
/// chars. Decoding those markers into a [`JSStr`] preserves the surrogates when
/// the AST is serialized back to ESTree.
pub fn read_js_str<'a>(
    value: &str,
    allocator: &impl GetAllocator<'a>,
) -> JSStr<'a> {
    decode_lone_surrogate_encoding(value, allocator)
        .unwrap_or_else(|| JSStr::from_str_in(value, allocator))
}

pub fn parse(input: &str) -> Result<Value, String> {
    let prepared: Cow<'_, str> = encode_lone_surrogate_escapes(input);

    sonic_rs::Deserializer::from_str(prepared.as_ref())
        .use_rawnumber()
        .deserialize::<Value>()
        .map_err(|error| error.to_string())
}
