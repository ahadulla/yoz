use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::encoding::{self, Encoding, LineEnding};

pub struct Buffer {
    pub lines: Vec<String>,
    pub file_path: Option<PathBuf>,
    pub modified: bool,
    pub encoding: Encoding,
    pub line_ending: LineEnding,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            file_path: None,
            modified: false,
            encoding: Encoding::Utf8,
            line_ending: if cfg!(windows) {
                LineEnding::Crlf
            } else {
                LineEnding::Lf
            },
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            let mut b = Self::new();
            b.file_path = Some(path);
            return Ok(b);
        }
        let bytes = fs::read(&path)?;
        let encoding = encoding::detect(&bytes);
        let line_ending = encoding::detect_line_ending(&bytes);
        let content = encoding::decode(&bytes, encoding);
        let lines = split_lines(&content);

        Ok(Self {
            lines,
            file_path: Some(path),
            modified: false,
            encoding,
            line_ending,
        })
    }

    pub fn reload_with_encoding(&mut self, enc: Encoding) -> io::Result<()> {
        let Some(path) = &self.file_path else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "fayl yo'q"));
        };
        let bytes = fs::read(path)?;
        let content = encoding::decode(&bytes, enc);
        self.lines = split_lines(&content);
        self.encoding = enc;
        self.line_ending = encoding::detect_line_ending(&bytes);
        self.modified = false;
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        let Some(path) = &self.file_path else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "fayl nomi yo'q"));
        };
        let content = self.lines.join(self.line_ending.as_str());
        let bytes = encoding::encode(&content, self.encoding)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, bytes)?;
        self.modified = false;
        Ok(())
    }

    pub fn file_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "[Nomsiz]".to_string())
    }

    pub fn insert_char(&mut self, row: usize, col: usize, ch: char) {
        if row >= self.lines.len() {
            return;
        }
        let line = &mut self.lines[row];
        let byte_idx = char_to_byte_idx(line, col);
        line.insert(byte_idx, ch);
        self.modified = true;
    }

    pub fn insert_newline(&mut self, row: usize, col: usize) {
        if row >= self.lines.len() {
            return;
        }
        let line = &mut self.lines[row];
        let byte_idx = char_to_byte_idx(line, col);
        let rest = line.split_off(byte_idx);
        self.lines.insert(row + 1, rest);
        self.modified = true;
    }

    pub fn delete_char_before(&mut self, row: usize, col: usize) -> (usize, usize) {
        if col > 0 {
            let line = &mut self.lines[row];
            let start = char_to_byte_idx(line, col - 1);
            let end = char_to_byte_idx(line, col);
            line.replace_range(start..end, "");
            self.modified = true;
            (row, col - 1)
        } else if row > 0 {
            let current = self.lines.remove(row);
            let prev_len = self.lines[row - 1].chars().count();
            self.lines[row - 1].push_str(&current);
            self.modified = true;
            (row - 1, prev_len)
        } else {
            (row, col)
        }
    }

    pub fn delete_char_at(&mut self, row: usize, col: usize) {
        let line_len = self.lines[row].chars().count();
        if col < line_len {
            let line = &mut self.lines[row];
            let start = char_to_byte_idx(line, col);
            let end = char_to_byte_idx(line, col + 1);
            line.replace_range(start..end, "");
            self.modified = true;
        } else if row + 1 < self.lines.len() {
            let next = self.lines.remove(row + 1);
            self.lines[row].push_str(&next);
            self.modified = true;
        }
    }

    pub fn line_len(&self, row: usize) -> usize {
        self.lines.get(row).map(|l| l.chars().count()).unwrap_or(0)
    }
}

fn split_lines(content: &str) -> Vec<String> {
    let mut lines: Vec<String> = content.split('\n').map(String::from).collect();
    for l in &mut lines {
        if l.ends_with('\r') {
            l.pop();
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}
