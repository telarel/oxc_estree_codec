use std::borrow::Cow;

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

/// Rewrite JSON `\uXXXX` escapes for unpaired surrogates into oxc's in-memory
/// lone-surrogate encoding (`U+FFFD` followed by 4 lowercase hex chars), which
/// oxc's ESTree serializer maps back to the original `\uXXXX` escape.
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

pub type Value = sonic_rs::Value;

pub fn parse(input: &str) -> Result<Value, String> {
    let prepared: Cow<'_, str> = encode_lone_surrogate_escapes(input);

    sonic_rs::Deserializer::from_str(prepared.as_ref())
        .use_rawnumber()
        .deserialize::<Value>()
        .map_err(|error| error.to_string())
}
