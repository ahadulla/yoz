use encoding_rs::{IBM866, UTF_8, WINDOWS_1251, WINDOWS_1252};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Cp1251,
    Cp1252,
    Cp866,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }
}

impl Encoding {
    pub fn all() -> [Encoding; 7] {
        [
            Encoding::Utf8,
            Encoding::Utf8Bom,
            Encoding::Utf16Le,
            Encoding::Utf16Be,
            Encoding::Cp1251,
            Encoding::Cp1252,
            Encoding::Cp866,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 BOM",
            Encoding::Utf16Le => "UTF-16 LE",
            Encoding::Utf16Be => "UTF-16 BE",
            Encoding::Cp1251 => "CP1251",
            Encoding::Cp1252 => "CP1252",
            Encoding::Cp866 => "CP866",
        }
    }
}

pub fn detect(bytes: &[u8]) -> Encoding {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Encoding::Utf8Bom;
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Encoding::Utf16Le;
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Encoding::Utf16Be;
    }

    let mut det = chardetng::EncodingDetector::new();
    det.feed(bytes, true);
    let guess = det.guess(None, true);

    if guess == UTF_8 {
        Encoding::Utf8
    } else if guess == WINDOWS_1251 {
        Encoding::Cp1251
    } else if guess == WINDOWS_1252 {
        Encoding::Cp1252
    } else if guess == IBM866 {
        Encoding::Cp866
    } else {
        Encoding::Utf8
    }
}

pub fn decode(bytes: &[u8], enc: Encoding) -> String {
    match enc {
        Encoding::Utf8 => {
            let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                &bytes[3..]
            } else {
                bytes
            };
            let (cow, _, _) = UTF_8.decode(content);
            cow.into_owned()
        }
        Encoding::Utf8Bom => {
            let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                &bytes[3..]
            } else {
                bytes
            };
            let (cow, _, _) = UTF_8.decode(content);
            cow.into_owned()
        }
        Encoding::Utf16Le => decode_utf16(bytes, true),
        Encoding::Utf16Be => decode_utf16(bytes, false),
        Encoding::Cp1251 => WINDOWS_1251.decode(bytes).0.into_owned(),
        Encoding::Cp1252 => WINDOWS_1252.decode(bytes).0.into_owned(),
        Encoding::Cp866 => IBM866.decode(bytes).0.into_owned(),
    }
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
    let start = if little_endian && bytes.starts_with(&[0xFF, 0xFE]) {
        2
    } else if !little_endian && bytes.starts_with(&[0xFE, 0xFF]) {
        2
    } else {
        0
    };
    let body = &bytes[start..];
    let mut units: Vec<u16> = Vec::with_capacity(body.len() / 2);
    let mut i = 0;
    while i + 1 < body.len() {
        let u = if little_endian {
            u16::from_le_bytes([body[i], body[i + 1]])
        } else {
            u16::from_be_bytes([body[i], body[i + 1]])
        };
        units.push(u);
        i += 2;
    }
    String::from_utf16_lossy(&units)
}

pub fn encode(text: &str, enc: Encoding) -> Result<Vec<u8>, String> {
    match enc {
        Encoding::Utf8 => Ok(text.as_bytes().to_vec()),
        Encoding::Utf8Bom => {
            let mut out = vec![0xEF, 0xBB, 0xBF];
            out.extend_from_slice(text.as_bytes());
            Ok(out)
        }
        Encoding::Utf16Le => {
            let mut out = vec![0xFF, 0xFE];
            for u in text.encode_utf16() {
                out.extend_from_slice(&u.to_le_bytes());
            }
            Ok(out)
        }
        Encoding::Utf16Be => {
            let mut out = vec![0xFE, 0xFF];
            for u in text.encode_utf16() {
                out.extend_from_slice(&u.to_be_bytes());
            }
            Ok(out)
        }
        Encoding::Cp1251 => encode_single_byte(text, WINDOWS_1251),
        Encoding::Cp1252 => encode_single_byte(text, WINDOWS_1252),
        Encoding::Cp866 => encode_single_byte(text, IBM866),
    }
}

fn encode_single_byte(
    text: &str,
    enc: &'static encoding_rs::Encoding,
) -> Result<Vec<u8>, String> {
    let (cow, _, had_errors) = enc.encode(text);
    if had_errors {
        return Err(format!(
            "Matn {} ga to'liq o'tkazilmadi (ba'zi belgilar qo'llab-quvvatlanmaydi)",
            enc.name()
        ));
    }
    Ok(cow.into_owned())
}

pub fn detect_line_ending(bytes: &[u8]) -> LineEnding {
    if bytes.windows(2).any(|w| w == b"\r\n") {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}
