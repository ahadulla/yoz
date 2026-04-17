use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};

use arboard::Clipboard;

use crate::buffer::Buffer;
use crate::encoding::Encoding;
use crate::history::{Action, History};
use crate::selection::{Pos, Selection};
use crate::terminal::Terminal;
use crate::ui::{View, draw_help, render};

pub struct Editor {
    buffer: Buffer,
    terminal: Terminal,
    view: View,
    cursor_row: usize,
    cursor_col: usize,
    status_msg: String,
    status_time: Instant,
    quit_confirm: bool,
    should_quit: bool,
    last_scroll: Option<Instant>,
    scroll_streak: usize,
    encoding_picker: bool,
    dragging_scrollbar: bool,
    last_click: Option<Instant>,
    show_help: bool,
    selection: Selection,
    clipboard: Option<Clipboard>,
    history: History,
    search_mode: SearchMode,
    search_query: String,
    replace_query: String,
    search_matches: Vec<Pos>,
    search_match_idx: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SearchMode {
    None,
    Find,
    Replace,
    ReplaceInput,
}

impl Editor {
    pub fn new(file_path: Option<String>) -> io::Result<Self> {
        let buffer = match file_path {
            Some(p) => Buffer::from_file(p)?,
            None => Buffer::new(),
        };
        let terminal = Terminal::enter()?;
        let (w, h) = Terminal::size()?;
        let view = View::new(w, h);
        Ok(Self {
            buffer,
            terminal,
            view,
            cursor_row: 0,
            cursor_col: 0,
            status_msg: String::from(
                "Ctrl+S saqlash  |  Ctrl+E encoding  |  Ctrl+L qator raqami  |  Ctrl+Q chiqish |  F1 Yordam",
            ),
            status_time: Instant::now(),
            quit_confirm: false,
            should_quit: false,
            last_scroll: None,
            scroll_streak: 0,
            encoding_picker: false,
            dragging_scrollbar: false,
            last_click: None,
            show_help: false,
            selection: Selection::new(),
            clipboard: Clipboard::new().ok(),
            history: History::new(),
            search_mode: SearchMode::None,
            search_query: String::new(),
            replace_query: String::new(),
            search_matches: Vec::new(),
            search_match_idx: None,
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.redraw()?;
        while !self.should_quit {
            let had_event = self.process_event()?;
            let expired = self.refresh_status_if_expired();
            if had_event || expired {
                self.redraw()?;
            }
        }
        Ok(())
    }

    fn redraw(&mut self) -> io::Result<()> {
        self.view.total_lines = self.buffer.lines.len();
        render(
            &mut self.terminal,
            &self.buffer,
            &self.view,
            self.cursor_row,
            self.cursor_col,
            &self.status_msg,
            &self.selection,
        )?;
        if self.show_help {
            draw_help(&mut self.terminal, &self.view)?;
        }
        Ok(())
    }

    fn cursor_pos(&self) -> Pos {
        Pos::new(self.cursor_row, self.cursor_col)
    }

    fn start_or_extend_selection(&mut self) {
        if !self.selection.is_active() {
            self.selection.start_at(self.cursor_pos());
        }
    }

    fn get_selected_text(&self) -> Option<String> {
        let (start, end) = self.selection.range(self.cursor_pos())?;
        let mut result = String::new();
        for row in start.row..=end.row {
            if row >= self.buffer.lines.len() {
                break;
            }
            let line = &self.buffer.lines[row];
            let chars: Vec<char> = line.chars().collect();
            let from = if row == start.row { start.col } else { 0 };
            let to = if row == end.row {
                end.col
            } else {
                chars.len()
            };
            let from = from.min(chars.len());
            let to = to.min(chars.len());
            let part: String = chars[from..to].iter().collect();
            result.push_str(&part);
            if row < end.row {
                result.push('\n');
            }
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection.range(self.cursor_pos()) else {
            return false;
        };
        self.history.begin_group();
        self.cursor_row = end.row;
        self.cursor_col = end.col;
        while (self.cursor_row, self.cursor_col) > (start.row, start.col) {
            self.backspace();
        }
        self.history.end_group();
        self.selection.clear();
        true
    }

    fn copy_selection(&mut self) {
        if let Some(text) = self.get_selected_text() {
            if let Some(cb) = &mut self.clipboard {
                let _ = cb.set_text(text);
            }
        }
    }

    fn cut_selection(&mut self) {
        self.copy_selection();
        self.delete_selection();
    }

    fn paste(&mut self) {
        let text = match &mut self.clipboard {
            Some(cb) => cb.get_text().ok(),
            None => None,
        };
        let Some(text) = text else { return };
        self.delete_selection();
        self.history.begin_group();
        for ch in text.chars() {
            if ch == '\n' {
                self.insert_newline();
            } else if ch != '\r' {
                self.insert_char(ch);
            }
        }
        self.history.end_group();
    }

    fn ensure_cursor_visible(&mut self) {
        self.view.scroll_to(self.cursor_row, self.cursor_col);
    }

    fn refresh_status_if_expired(&mut self) -> bool {
        if self.encoding_picker {
            return false;
        }
        if self.status_time.elapsed() > Duration::from_secs(5) && !self.status_msg.is_empty() {
            self.status_msg.clear();
            return true;
        }
        false
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
        self.status_time = Instant::now();
    }

    fn process_event(&mut self) -> io::Result<bool> {
        if !event::poll(Duration::from_millis(500))? {
            return Ok(false);
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key(key)?;
                Ok(true)
            }
            Event::Mouse(m) => {
                self.handle_mouse(m);
                Ok(true)
            }
            Event::Resize(w, h) => {
                self.view.resize(w, h);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn handle_mouse(&mut self, m: MouseEvent) {
        match m.kind {
            MouseEventKind::ScrollUp => {
                let step = self.scroll_step();
                self.scroll_lines(-(step as isize));
            }
            MouseEventKind::ScrollDown => {
                let step = self.scroll_step();
                self.scroll_lines(step as isize);
            }
            MouseEventKind::Down(_) => {
                if m.column == self.view.scrollbar_x() {
                    self.dragging_scrollbar = true;
                    self.scrollbar_jump(m.row);
                } else {
                    let now = Instant::now();
                    let double = matches!(self.last_click, Some(t) if now.duration_since(t) < Duration::from_millis(400));
                    self.last_click = Some(now);

                    if double {
                        self.select_word_at_cursor();
                    } else {
                        self.selection.clear();
                        self.mouse_click(m.column, m.row);
                        self.selection.start_at(self.cursor_pos());
                    }
                }
            }
            MouseEventKind::Drag(_) => {
                if self.dragging_scrollbar {
                    self.scrollbar_jump(m.row);
                } else {
                    self.mouse_click(m.column, m.row);
                }
            }
            MouseEventKind::Up(_) => {
                self.dragging_scrollbar = false;
                if self.selection.range(self.cursor_pos()).is_none() {
                    self.selection.clear();
                }
            }
            MouseEventKind::Moved => {
                let hover = m.column == self.view.scrollbar_x();
                if hover != self.view.scrollbar_hover {
                    self.view.scrollbar_hover = hover;
                }
            }
            _ => {}
        }
    }

    fn scrollbar_jump(&mut self, screen_y: u16) {
        let total = self.buffer.lines.len();
        let text_rows = self.view.text_rows() as usize;
        if text_rows == 0 || total <= text_rows {
            return;
        }
        let start = self.view.text_start_row();
        let rel = (screen_y.saturating_sub(start)) as usize;
        let max_offset = total.saturating_sub(text_rows);
        let new_offset = if rel >= text_rows.saturating_sub(1) {
            max_offset
        } else {
            (rel * max_offset) / (text_rows.saturating_sub(1)).max(1)
        };
        self.view.row_offset = new_offset.min(max_offset);
    }

    fn scroll_step(&mut self) -> usize {
        let now = Instant::now();
        let fast = matches!(self.last_scroll, Some(t) if now.duration_since(t) < Duration::from_millis(80));
        self.scroll_streak = if fast { self.scroll_streak + 1 } else { 1 };
        self.last_scroll = Some(now);
        match self.scroll_streak {
            0..=2 => 1,
            3..=5 => 3,
            6..=9 => 8,
            10..=14 => 14,
            _ => 20,
        }
    }

    fn scroll_lines(&mut self, delta: isize) {
        let total = self.buffer.lines.len();
        if total == 0 {
            return;
        }
        let text_rows = self.view.text_rows() as usize;
        let max_offset = total.saturating_sub(text_rows.max(1)) as isize;

        let old_offset = self.view.row_offset as isize;
        let requested = old_offset + delta;
        let new_offset = requested.clamp(0, max_offset);
        self.view.row_offset = new_offset as usize;

        let leftover = requested - new_offset;
        if leftover != 0 {
            let new_cursor =
                (self.cursor_row as isize + leftover).clamp(0, (total - 1) as isize);
            self.cursor_row = new_cursor as usize;
            self.clamp_col();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> io::Result<()> {
        if self.show_help {
            self.show_help = false;
            return Ok(());
        }

        if self.encoding_picker {
            self.handle_picker_key(key);
            return Ok(());
        }

        if self.search_mode != SearchMode::None {
            self.handle_search_key(key);
            return Ok(());
        }

        self.snap_cursor_to_view();

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        if key.code == KeyCode::F(1) {
            self.show_help = true;
            return Ok(());
        }

        // Ctrl+Shift combinations
        if ctrl && shift {
            match key.code {
                KeyCode::Left => {
                    self.start_or_extend_selection();
                    self.move_word_left();
                }
                KeyCode::Right => {
                    self.start_or_extend_selection();
                    self.move_word_right();
                }
                KeyCode::Home => {
                    self.start_or_extend_selection();
                    self.cursor_row = 0;
                    self.cursor_col = 0;
                }
                KeyCode::End => {
                    self.start_or_extend_selection();
                    self.cursor_row = self.buffer.lines.len().saturating_sub(1);
                    self.cursor_col = self.buffer.line_len(self.cursor_row);
                }
                _ => {}
            }
            self.ensure_cursor_visible();
            return Ok(());
        }

        // Ctrl combinations
        if ctrl {
            match key.code {
                KeyCode::Char('e') => {
                    self.open_encoding_picker();
                    return Ok(());
                }
                KeyCode::Char('l') => {
                    self.view.show_line_numbers = !self.view.show_line_numbers;
                    let state = if self.view.show_line_numbers {
                        "yoqildi"
                    } else {
                        "o'chirildi"
                    };
                    self.set_status(format!("Qator raqamlari {state}"));
                    return Ok(());
                }
                KeyCode::Char('q') => {
                    if self.buffer.modified && !self.quit_confirm {
                        self.quit_confirm = true;
                        self.set_status(
                            "Saqlanmagan o'zgarishlar bor! Chiqish uchun Ctrl+Q ni yana bosing.",
                        );
                        return Ok(());
                    }
                    self.should_quit = true;
                    return Ok(());
                }
                KeyCode::Char('s') => {
                    self.save()?;
                    return Ok(());
                }
                KeyCode::Char('a') => {
                    self.selection.start_at(Pos::new(0, 0));
                    self.cursor_row = self.buffer.lines.len().saturating_sub(1);
                    self.cursor_col = self.buffer.line_len(self.cursor_row);
                }
                KeyCode::Char('c') => {
                    self.copy_selection();
                    self.set_status("Nusxa olindi");
                    return Ok(());
                }
                KeyCode::Char('x') => {
                    self.cut_selection();
                    self.set_status("Kesildi");
                }
                KeyCode::Char('v') => {
                    self.paste();
                }
                KeyCode::Char('f') => {
                    self.search_mode = SearchMode::Find;
                    self.search_query.clear();
                    self.search_matches.clear();
                    self.search_match_idx = None;
                    self.set_status("Qidirish: ");
                    return Ok(());
                }
                KeyCode::Char('h') => {
                    self.search_mode = SearchMode::Replace;
                    self.search_query.clear();
                    self.replace_query.clear();
                    self.search_matches.clear();
                    self.search_match_idx = None;
                    self.set_status("Qidirish (almashtirish): ");
                    return Ok(());
                }
                KeyCode::Char('n') => {
                    self.goto_next_match();
                }
                KeyCode::Char('p') => {
                    self.goto_prev_match();
                }
                KeyCode::Char('z') => {
                    self.undo();
                }
                KeyCode::Char('y') => {
                    self.redo();
                }
                KeyCode::Char('d') => {
                    let row = self.cursor_row;
                    if row < self.buffer.lines.len() {
                        let dup = self.buffer.lines[row].clone();
                        self.buffer.lines.insert(row + 1, dup);
                        self.buffer.modified = true;
                        self.cursor_row += 1;
                    }
                }
                KeyCode::Char('k') => {
                    let len = self.buffer.line_len(self.cursor_row);
                    if self.cursor_col < len {
                        while self.cursor_col < self.buffer.line_len(self.cursor_row) {
                            self.buffer
                                .delete_char_at(self.cursor_row, self.cursor_col);
                        }
                    }
                }
                KeyCode::Home => {
                    self.selection.clear();
                    self.cursor_row = 0;
                    self.cursor_col = 0;
                }
                KeyCode::End => {
                    self.selection.clear();
                    self.cursor_row = self.buffer.lines.len().saturating_sub(1);
                    self.cursor_col = self.buffer.line_len(self.cursor_row);
                }
                KeyCode::Left => {
                    self.selection.clear();
                    self.move_word_left();
                }
                KeyCode::Right => {
                    self.selection.clear();
                    self.move_word_right();
                }
                KeyCode::Up => self.scroll_lines(-1),
                KeyCode::Down => self.scroll_lines(1),
                KeyCode::Backspace => {
                    if self.selection.is_active() {
                        self.delete_selection();
                    } else {
                        self.delete_word_left();
                    }
                }
                KeyCode::Delete => {
                    if self.selection.is_active() {
                        self.delete_selection();
                    } else {
                        self.delete_word_right();
                    }
                }
                _ => {}
            }
            self.ensure_cursor_visible();
            return Ok(());
        }

        // Shift combinations (selection)
        if shift {
            match key.code {
                KeyCode::Left => {
                    self.start_or_extend_selection();
                    self.move_left();
                }
                KeyCode::Right => {
                    self.start_or_extend_selection();
                    self.move_right();
                }
                KeyCode::Up => {
                    self.start_or_extend_selection();
                    self.move_up();
                }
                KeyCode::Down => {
                    self.start_or_extend_selection();
                    self.move_down();
                }
                KeyCode::Home => {
                    self.start_or_extend_selection();
                    self.cursor_col = 0;
                }
                KeyCode::End => {
                    self.start_or_extend_selection();
                    self.cursor_col = self.buffer.line_len(self.cursor_row);
                }
                KeyCode::PageUp => {
                    self.start_or_extend_selection();
                    let step = self.view.text_rows() as usize;
                    self.cursor_row = self.cursor_row.saturating_sub(step);
                    self.clamp_col();
                }
                KeyCode::PageDown => {
                    self.start_or_extend_selection();
                    let step = self.view.text_rows() as usize;
                    self.cursor_row =
                        (self.cursor_row + step).min(self.buffer.lines.len() - 1);
                    self.clamp_col();
                }
                KeyCode::Char(c) => {
                    self.delete_selection();
                    self.insert_char(c);
                }
                _ => {}
            }
            self.ensure_cursor_visible();
            return Ok(());
        }

        // No modifier
        self.quit_confirm = false;

        match key.code {
            KeyCode::Char(c) => {
                self.delete_selection();
                self.insert_char(c);
            }
            KeyCode::Enter => {
                self.delete_selection();
                self.insert_newline();
            }
            KeyCode::Backspace => {
                if !self.delete_selection() {
                    self.backspace();
                }
            }
            KeyCode::Delete => {
                if !self.delete_selection() {
                    self.delete();
                }
            }
            KeyCode::Left => {
                self.selection.clear();
                self.move_left();
            }
            KeyCode::Right => {
                self.selection.clear();
                self.move_right();
            }
            KeyCode::Up => {
                self.selection.clear();
                self.move_up();
            }
            KeyCode::Down => {
                self.selection.clear();
                self.move_down();
            }
            KeyCode::Home => {
                self.selection.clear();
                self.cursor_col = 0;
            }
            KeyCode::End => {
                self.selection.clear();
                self.cursor_col = self.buffer.line_len(self.cursor_row);
            }
            KeyCode::PageUp => {
                self.selection.clear();
                let step = self.view.text_rows() as usize;
                self.cursor_row = self.cursor_row.saturating_sub(step);
                self.clamp_col();
            }
            KeyCode::PageDown => {
                self.selection.clear();
                let step = self.view.text_rows() as usize;
                self.cursor_row = (self.cursor_row + step).min(self.buffer.lines.len() - 1);
                self.clamp_col();
            }
            KeyCode::Tab => {
                self.delete_selection();
                for _ in 0..4 {
                    self.insert_char(' ');
                }
            }
            KeyCode::Esc => {
                self.selection.clear();
            }
            _ => {}
        }
        self.ensure_cursor_visible();
        Ok(())
    }

    fn save(&mut self) -> io::Result<()> {
        if self.buffer.file_path.is_none() {
            self.set_status("Saqlash uchun fayl nomi yo'q (yoz <fayl> bilan oching)");
            return Ok(());
        }
        match self.buffer.save() {
            Ok(()) => self.set_status(format!("Saqlandi: {}", self.buffer.file_name())),
            Err(e) => self.set_status(format!("Saqlashda xato: {e}")),
        }
        Ok(())
    }

    fn insert_char(&mut self, c: char) {
        self.history.push(Action::InsertChar {
            row: self.cursor_row,
            col: self.cursor_col,
            ch: c,
        });
        self.buffer.insert_char(self.cursor_row, self.cursor_col, c);
        self.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        self.history.push(Action::InsertNewline {
            row: self.cursor_row,
            col: self.cursor_col,
        });
        self.buffer
            .insert_newline(self.cursor_row, self.cursor_col);
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    fn backspace(&mut self) {
        if self.cursor_col == 0 && self.cursor_row == 0 {
            return;
        }
        if self.cursor_col > 0 {
            let ch = self.buffer.lines[self.cursor_row]
                .chars()
                .nth(self.cursor_col - 1)
                .unwrap_or(' ');
            self.history.push(Action::DeleteChar {
                row: self.cursor_row,
                col: self.cursor_col - 1,
                ch,
            });
        } else {
            self.history.push(Action::DeleteNewline {
                row: self.cursor_row - 1,
                col: self.buffer.line_len(self.cursor_row - 1),
            });
        }
        let (r, c) = self
            .buffer
            .delete_char_before(self.cursor_row, self.cursor_col);
        self.cursor_row = r;
        self.cursor_col = c;
    }

    fn delete(&mut self) {
        let line_len = self.buffer.line_len(self.cursor_row);
        if self.cursor_col < line_len {
            let ch = self.buffer.lines[self.cursor_row]
                .chars()
                .nth(self.cursor_col)
                .unwrap_or(' ');
            self.history.push(Action::DeleteChar {
                row: self.cursor_row,
                col: self.cursor_col,
                ch,
            });
        } else if self.cursor_row + 1 < self.buffer.lines.len() {
            self.history.push(Action::DeleteNewline {
                row: self.cursor_row,
                col: self.cursor_col,
            });
        }
        self.buffer.delete_char_at(self.cursor_row, self.cursor_col);
    }

    fn undo(&mut self) {
        let Some(actions) = self.history.undo() else {
            return;
        };
        for action in actions.iter().rev() {
            match action {
                Action::InsertChar { row, col, .. } => {
                    self.buffer.delete_char_at(*row, *col);
                    self.cursor_row = *row;
                    self.cursor_col = *col;
                }
                Action::DeleteChar { row, col, ch } => {
                    self.buffer.insert_char(*row, *col, *ch);
                    self.cursor_row = *row;
                    self.cursor_col = *col + 1;
                }
                Action::InsertNewline { row, col } => {
                    self.cursor_row = row + 1;
                    self.cursor_col = 0;
                    self.buffer
                        .delete_char_before(self.cursor_row, self.cursor_col);
                    self.cursor_row = *row;
                    self.cursor_col = *col;
                }
                Action::DeleteNewline { row, col } => {
                    self.buffer.insert_newline(*row, *col);
                    self.cursor_row = row + 1;
                    self.cursor_col = 0;
                }
            }
        }
    }

    fn redo(&mut self) {
        let Some(actions) = self.history.redo() else {
            return;
        };
        for action in actions.iter() {
            match action {
                Action::InsertChar { row, col, ch } => {
                    self.buffer.insert_char(*row, *col, *ch);
                    self.cursor_row = *row;
                    self.cursor_col = *col + 1;
                }
                Action::DeleteChar { row, col, .. } => {
                    self.buffer.delete_char_at(*row, *col);
                    self.cursor_row = *row;
                    self.cursor_col = *col;
                }
                Action::InsertNewline { row, col } => {
                    self.buffer.insert_newline(*row, *col);
                    self.cursor_row = row + 1;
                    self.cursor_col = 0;
                }
                Action::DeleteNewline { row, col } => {
                    self.cursor_row = row + 1;
                    self.cursor_col = 0;
                    self.buffer
                        .delete_char_before(self.cursor_row, self.cursor_col);
                    self.cursor_row = *row;
                    self.cursor_col = *col;
                }
            }
        }
    }

    fn move_word_left(&mut self) {
        if self.cursor_col == 0 {
            if self.cursor_row > 0 {
                self.cursor_row -= 1;
                self.cursor_col = self.buffer.line_len(self.cursor_row);
            }
            return;
        }
        let line = &self.buffer.lines[self.cursor_row];
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor_col;
        while col > 0 && !chars[col - 1].is_alphanumeric() {
            col -= 1;
        }
        while col > 0 && chars[col - 1].is_alphanumeric() {
            col -= 1;
        }
        self.cursor_col = col;
    }

    fn move_word_right(&mut self) {
        let line_len = self.buffer.line_len(self.cursor_row);
        if self.cursor_col >= line_len {
            if self.cursor_row + 1 < self.buffer.lines.len() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
            return;
        }
        let line = &self.buffer.lines[self.cursor_row];
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor_col;
        while col < chars.len() && chars[col].is_alphanumeric() {
            col += 1;
        }
        while col < chars.len() && !chars[col].is_alphanumeric() {
            col += 1;
        }
        self.cursor_col = col;
    }

    fn delete_word_left(&mut self) {
        if self.cursor_col == 0 && self.cursor_row == 0 {
            return;
        }
        if self.cursor_col == 0 {
            self.backspace();
            return;
        }
        let target = {
            let line = &self.buffer.lines[self.cursor_row];
            let chars: Vec<char> = line.chars().collect();
            let mut col = self.cursor_col;
            while col > 0 && !chars[col - 1].is_alphanumeric() {
                col -= 1;
            }
            while col > 0 && chars[col - 1].is_alphanumeric() {
                col -= 1;
            }
            col
        };
        self.history.begin_group();
        while self.cursor_col > target {
            self.backspace();
        }
        self.history.end_group();
    }

    fn delete_word_right(&mut self) {
        let line_len = self.buffer.line_len(self.cursor_row);
        if self.cursor_col >= line_len && self.cursor_row + 1 >= self.buffer.lines.len() {
            return;
        }
        if self.cursor_col >= line_len {
            self.delete();
            return;
        }
        let target = {
            let line = &self.buffer.lines[self.cursor_row];
            let chars: Vec<char> = line.chars().collect();
            let mut col = self.cursor_col;
            while col < chars.len() && chars[col].is_alphanumeric() {
                col += 1;
            }
            while col < chars.len() && !chars[col].is_alphanumeric() {
                col += 1;
            }
            col
        };
        self.history.begin_group();
        while self.cursor_col < target && self.cursor_col < self.buffer.line_len(self.cursor_row) {
            self.delete();
        }
        self.history.end_group();
    }

    fn mouse_click(&mut self, col: u16, row: u16) {
        let start_row = self.view.text_start_row();
        if row < start_row {
            return;
        }
        let gutter = self.view.gutter_width();
        if col < gutter {
            return;
        }
        if col >= self.view.scrollbar_x() {
            return;
        }

        let file_row = self.view.row_offset + (row - start_row) as usize;
        let file_col = self.view.col_offset + (col - gutter) as usize;

        let total = self.buffer.lines.len();
        if file_row < total {
            self.cursor_row = file_row;
            self.cursor_col = file_col.min(self.buffer.line_len(self.cursor_row));
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.buffer.line_len(self.cursor_row);
        }
    }

    fn move_right(&mut self) {
        let len = self.buffer.line_len(self.cursor_row);
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.buffer.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_col();
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 < self.buffer.lines.len() {
            self.cursor_row += 1;
            self.clamp_col();
        }
    }

    fn open_encoding_picker(&mut self) {
        self.encoding_picker = true;
        let msg = "Encoding: [1]UTF-8  [2]UTF-8 BOM  [3]UTF-16 LE  [4]UTF-16 BE  [5]CP1251  [6]CP1252  [7]CP866   Esc=bekor";
        self.set_status(msg);
    }

    fn handle_picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.encoding_picker = false;
                self.set_status("");
            }
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let idx = c as usize - '1' as usize;
                let all = Encoding::all();
                if idx < all.len() {
                    let enc = all[idx];
                    self.encoding_picker = false;
                    self.apply_encoding(enc);
                }
            }
            _ => {}
        }
    }

    fn apply_encoding(&mut self, enc: Encoding) {
        if self.buffer.modified || self.buffer.file_path.is_none() {
            self.buffer.encoding = enc;
            self.set_status(format!("Saqlash encodingi: {}", enc.name()));
            return;
        }
        match self.buffer.reload_with_encoding(enc) {
            Ok(()) => {
                self.cursor_row = 0;
                self.cursor_col = 0;
                self.view.row_offset = 0;
                self.view.col_offset = 0;
                self.set_status(format!("Qayta o'qildi: {}", enc.name()));
            }
            Err(e) => self.set_status(format!("Xato: {e}")),
        }
    }

    fn snap_cursor_to_view(&mut self) {
        let text_rows = self.view.text_rows() as usize;
        if text_rows == 0 {
            return;
        }
        let total = self.buffer.lines.len();
        if total == 0 {
            return;
        }
        let view_start = self.view.row_offset;
        let view_end = (view_start + text_rows - 1).min(total - 1);

        if self.cursor_row < view_start {
            self.cursor_row = view_start.min(total - 1);
            self.cursor_col = 0;
        } else if self.cursor_row > view_end {
            self.cursor_row = view_end;
            self.cursor_col = 0;
        }
        self.clamp_col();
    }

    fn select_word_at_cursor(&mut self) {
        let row = self.cursor_row;
        if row >= self.buffer.lines.len() {
            return;
        }
        let chars: Vec<char> = self.buffer.lines[row].chars().collect();
        if chars.is_empty() {
            return;
        }
        let col = self.cursor_col.min(chars.len().saturating_sub(1));
        let is_word = chars[col].is_alphanumeric() || chars[col] == '_';
        let check = if is_word {
            |c: char| c.is_alphanumeric() || c == '_'
        } else {
            |c: char| !c.is_alphanumeric() && c != '_' && !c.is_whitespace()
        };

        let mut start = col;
        while start > 0 && check(chars[start - 1]) {
            start -= 1;
        }
        let mut end = col;
        while end < chars.len() && check(chars[end]) {
            end += 1;
        }

        self.selection.start_at(Pos::new(row, start));
        self.cursor_row = row;
        self.cursor_col = end;
    }

    fn clamp_col(&mut self) {
        let len = self.buffer.line_len(self.cursor_row);
        if self.cursor_col > len {
            self.cursor_col = len;
        }
    }

    // --- Search / Replace ---

    fn handle_search_key(&mut self, key: KeyEvent) {
        match self.search_mode {
            SearchMode::Find => self.handle_find_input(key),
            SearchMode::Replace => self.handle_replace_search_input(key),
            SearchMode::ReplaceInput => self.handle_replace_text_input(key),
            SearchMode::None => {}
        }
    }

    fn handle_find_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.search_mode = SearchMode::None;
                self.search_matches.clear();
                self.set_status("");
            }
            KeyCode::Enter => {
                self.perform_search();
                if !self.search_matches.is_empty() {
                    self.search_mode = SearchMode::None;
                    self.goto_next_match();
                } else {
                    self.set_status(format!("\"{}\" topilmadi", self.search_query));
                    self.search_mode = SearchMode::None;
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search_status();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_search_status();
            }
            _ => {}
        }
    }

    fn handle_replace_search_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.search_mode = SearchMode::None;
                self.set_status("");
            }
            KeyCode::Enter => {
                self.perform_search();
                if self.search_matches.is_empty() {
                    self.set_status(format!("\"{}\" topilmadi", self.search_query));
                    self.search_mode = SearchMode::None;
                } else {
                    self.search_mode = SearchMode::ReplaceInput;
                    self.replace_query.clear();
                    self.update_replace_status();
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.set_status(format!("Qidirish (almashtirish): {}", self.search_query));
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.set_status(format!("Qidirish (almashtirish): {}", self.search_query));
            }
            _ => {}
        }
    }

    fn handle_replace_text_input(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                self.search_mode = SearchMode::None;
                self.set_status("");
            }
            KeyCode::Enter => {
                self.replace_next();
                self.update_replace_status();
            }
            KeyCode::Char('a') if ctrl => {
                self.replace_all();
                self.search_mode = SearchMode::None;
            }
            KeyCode::Backspace => {
                self.replace_query.pop();
                self.update_replace_status();
            }
            KeyCode::Char(c) => {
                self.replace_query.push(c);
                self.update_replace_status();
            }
            _ => {}
        }
    }

    fn update_search_status(&mut self) {
        self.set_status(format!("Qidirish: {}", self.search_query));
    }

    fn update_replace_status(&mut self) {
        let count = self.search_matches.len();
        self.set_status(format!(
            "Almashtirish: {} -> {}  ({} ta)  Enter=keyingi  Ctrl+A=hammasi  Esc=bekor",
            self.search_query, self.replace_query, count
        ));
    }

    fn perform_search(&mut self) {
        self.search_matches.clear();
        self.search_match_idx = None;
        if self.search_query.is_empty() {
            return;
        }
        let query: Vec<char> = self.search_query.chars().collect();
        let qlen = query.len();
        for (row, line) in self.buffer.lines.iter().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            if chars.len() < qlen {
                continue;
            }
            for col in 0..=chars.len() - qlen {
                if chars[col..col + qlen] == query[..] {
                    self.search_matches.push(Pos::new(row, col));
                }
            }
        }
    }

    fn goto_next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        let cursor = self.cursor_pos();
        let idx = match self.search_match_idx {
            Some(i) => {
                let next = i + 1;
                if next >= self.search_matches.len() {
                    0
                } else {
                    next
                }
            }
            None => {
                self.search_matches
                    .iter()
                    .position(|p| *p >= cursor)
                    .unwrap_or(0)
            }
        };
        self.search_match_idx = Some(idx);
        let pos = self.search_matches[idx];
        self.cursor_row = pos.row;
        self.cursor_col = pos.col;
        self.selection.start_at(pos);
        let end_col = pos.col + self.search_query.chars().count();
        self.cursor_col = end_col;
        self.ensure_cursor_visible();
        let total = self.search_matches.len();
        self.set_status(format!(
            "\"{}\" — {}/{} topildi  Ctrl+N=keyingi  Ctrl+P=oldingi",
            self.search_query,
            idx + 1,
            total
        ));
    }

    fn goto_prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        let idx = match self.search_match_idx {
            Some(0) | None => self.search_matches.len() - 1,
            Some(i) => i - 1,
        };
        self.search_match_idx = Some(idx);
        let pos = self.search_matches[idx];
        self.cursor_row = pos.row;
        self.cursor_col = pos.col;
        self.selection.start_at(pos);
        self.cursor_col = pos.col + self.search_query.chars().count();
        self.ensure_cursor_visible();
        let total = self.search_matches.len();
        self.set_status(format!(
            "\"{}\" — {}/{} topildi  Ctrl+N=keyingi  Ctrl+P=oldingi",
            self.search_query,
            idx + 1,
            total
        ));
    }

    fn replace_next(&mut self) {
        if self.search_matches.is_empty() {
            self.set_status("Hech narsa topilmadi");
            self.search_mode = SearchMode::None;
            return;
        }
        let cursor = self.cursor_pos();
        let idx = self
            .search_matches
            .iter()
            .position(|p| *p >= cursor)
            .unwrap_or(0);
        let pos = self.search_matches[idx];
        self.cursor_row = pos.row;
        self.cursor_col = pos.col;
        let qlen = self.search_query.chars().count();
        let replacement: Vec<char> = self.replace_query.chars().collect();
        self.history.begin_group();
        for _ in 0..qlen {
            self.delete();
        }
        for ch in replacement {
            self.insert_char(ch);
        }
        self.history.end_group();
        self.perform_search();
        if self.search_matches.is_empty() {
            self.set_status("Almashtirish tugadi");
            self.search_mode = SearchMode::None;
        }
    }

    fn replace_all(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        let count = self.search_matches.len();
        let qlen = self.search_query.chars().count();
        let replacement: Vec<char> = self.replace_query.chars().collect();
        self.history.begin_group();
        while !self.search_matches.is_empty() {
            let pos = *self.search_matches.last().unwrap();
            self.cursor_row = pos.row;
            self.cursor_col = pos.col;
            for _ in 0..qlen {
                self.delete();
            }
            for &ch in &replacement {
                self.insert_char(ch);
            }
            self.perform_search();
        }
        self.history.end_group();
        self.set_status(format!("{count} ta almashtirildi"));
    }
}
