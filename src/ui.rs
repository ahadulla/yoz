use std::io;

use crate::buffer::Buffer;
use crate::selection::{Pos, Selection};
use crate::terminal::{Color, Terminal};

pub struct View {
    pub width: u16,
    pub height: u16,
    pub row_offset: usize,
    pub col_offset: usize,
    pub show_line_numbers: bool,
    pub total_lines: usize,
    pub scrollbar_hover: bool,
}

impl View {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            row_offset: 0,
            col_offset: 0,
            show_line_numbers: true,
            total_lines: 1,
            scrollbar_hover: false,
        }
    }

    pub fn resize(&mut self, w: u16, h: u16) {
        self.width = w;
        self.height = h;
    }

    pub fn text_rows(&self) -> u16 {
        self.height.saturating_sub(2)
    }

    pub fn text_start_row(&self) -> u16 {
        1
    }

    pub fn gutter_width(&self) -> u16 {
        if !self.show_line_numbers {
            return 0;
        }
        let digits = digit_count(self.total_lines.max(1));
        digits as u16 + 2
    }

    pub fn text_width(&self) -> u16 {
        self.width
            .saturating_sub(1)
            .saturating_sub(self.gutter_width())
    }

    pub fn scroll_to(&mut self, cursor_row: usize, cursor_col: usize) {
        let text_rows = self.text_rows() as usize;
        if cursor_row < self.row_offset {
            self.row_offset = cursor_row;
        } else if cursor_row >= self.row_offset + text_rows {
            self.row_offset = cursor_row + 1 - text_rows;
        }

        let width = self.text_width() as usize;
        if cursor_col < self.col_offset {
            self.col_offset = cursor_col;
        } else if cursor_col >= self.col_offset + width {
            self.col_offset = cursor_col + 1 - width;
        }
    }
}

impl View {
    pub fn scrollbar_thumb(&self, total_lines: usize) -> (usize, usize) {
        let text_rows = self.text_rows() as usize;
        let total = total_lines.max(1);
        if total <= text_rows {
            (0, text_rows)
        } else {
            let thumb_size = ((text_rows * text_rows) / total).max(1);
            let max_offset = total - text_rows;
            let track = text_rows - thumb_size;
            let pos = (self.row_offset * track) / max_offset.max(1);
            (pos, pos + thumb_size)
        }
    }

    pub fn scrollbar_x(&self) -> u16 {
        self.width.saturating_sub(1)
    }
}

fn digit_count(n: usize) -> usize {
    let mut n = n;
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    d
}

pub fn render(
    term: &mut Terminal,
    buffer: &Buffer,
    view: &View,
    cursor_row: usize,
    cursor_col: usize,
    status_msg: &str,
    selection: &Selection,
) -> io::Result<()> {
    term.begin_sync()?;
    let cursor = Pos::new(cursor_row, cursor_col);
    draw_text(term, buffer, view, selection, cursor)?;
    draw_scrollbar(term, buffer, view)?;
    draw_status_bar(term, buffer, view, cursor_row, cursor_col)?;
    draw_message_bar(term, view, status_msg)?;

    let screen_x =
        (cursor_col.saturating_sub(view.col_offset)) as u16 + view.gutter_width();
    let screen_y = (cursor_row.saturating_sub(view.row_offset)) as u16 + view.text_start_row();
    term.move_to(screen_x, screen_y)?;
    term.end_sync()?;
    term.flush()?;
    Ok(())
}

fn draw_text(
    term: &mut Terminal,
    buffer: &Buffer,
    view: &View,
    selection: &Selection,
    cursor: Pos,
) -> io::Result<()> {
    let text_rows = view.text_rows();
    let start = view.text_start_row();
    let width = view.text_width() as usize;
    let gutter = view.gutter_width() as usize;
    let gutter_fg = Color::Rgb { r: 110, g: 110, b: 110 };
    let sel_bg = Color::Rgb { r: 50, g: 80, b: 140 };
    let has_sel = selection.is_active();

    for screen_row in 0..text_rows {
        term.move_to(0, start + screen_row)?;
        term.clear_line()?;
        let file_row = view.row_offset + screen_row as usize;
        let has_line = buffer.lines.get(file_row).is_some();

        if gutter > 0 {
            if has_line {
                term.set_colors(gutter_fg, Color::Reset)?;
                let num = format!("{:>width$} ", file_row + 1, width = gutter - 1);
                term.write(&num)?;
                term.reset_colors()?;
            } else {
                term.write(&" ".repeat(gutter))?;
            }
        }

        if let Some(line) = buffer.lines.get(file_row) {
            if has_sel {
                let chars: Vec<char> = line.chars().skip(view.col_offset).take(width).collect();
                let mut in_sel = false;
                for (i, ch) in chars.iter().enumerate() {
                    let col = view.col_offset + i;
                    let selected = selection.contains(cursor, file_row, col);
                    if selected != in_sel {
                        if selected {
                            term.set_colors(Color::White, sel_bg)?;
                        } else {
                            term.reset_colors()?;
                        }
                        in_sel = selected;
                    }
                    let mut buf = [0u8; 4];
                    term.write(ch.encode_utf8(&mut buf))?;
                }
                if in_sel {
                    term.reset_colors()?;
                }
            } else {
                let visible: String = line.chars().skip(view.col_offset).take(width).collect();
                term.write(&visible)?;
            }
        }
    }
    Ok(())
}

fn draw_scrollbar(term: &mut Terminal, buffer: &Buffer, view: &View) -> io::Result<()> {
    let text_rows = view.text_rows() as usize;
    if text_rows == 0 || view.width == 0 {
        return Ok(());
    }
    let x = view.scrollbar_x();
    let start = view.text_start_row();
    let (thumb_start, thumb_end) = view.scrollbar_thumb(buffer.lines.len());
    let thumb_len = thumb_end - thumb_start;

    let thumb_color = Color::Rgb { r: 130, g: 130, b: 130 };

    if view.scrollbar_hover {
        for i in 0..text_rows {
            term.move_to(x, start + i as u16)?;
            if i >= thumb_start && i < thumb_end {
                term.set_colors(thumb_color, Color::Reset)?;
                if thumb_len == 1 {
                    term.write("\u{2586}")?;
                } else if i == thumb_start {
                    term.write("\u{2584}")?;
                } else if i == thumb_end - 1 {
                    term.write("\u{2580}")?;
                } else {
                    term.write("\u{2588}")?;
                }
                term.reset_colors()?;
            } else {
                term.write(" ")?;
            }
        }
    } else {
        let thin_color = Color::Rgb { r: 90, g: 90, b: 90 };
        for i in 0..text_rows {
            term.move_to(x, start + i as u16)?;
            if i >= thumb_start && i < thumb_end {
                term.set_colors(thin_color, Color::Reset)?;
                term.write("\u{2502}")?;
                term.reset_colors()?;
            } else {
                term.write(" ")?;
            }
        }
    }
    Ok(())
}

fn draw_status_bar(
    term: &mut Terminal,
    buffer: &Buffer,
    view: &View,
    cursor_row: usize,
    cursor_col: usize,
) -> io::Result<()> {
    let status_bg = Color::Rgb { r: 55, g: 55, b: 55 };
    let status_fg = Color::White;
    term.move_to(0, 0)?;
    term.set_colors(status_fg, status_bg)?;
    term.clear_line()?;

    let modified = if buffer.modified { " *" } else { "" };
    let left = format!(" {}{}  ", buffer.file_name(), modified);
    let right = format!(
        "{}  {}:{}  {} qator ",
        buffer.encoding.name(),
        cursor_row + 1,
        cursor_col + 1,
        buffer.lines.len()
    );

    let width = view.width as usize;
    let mut status = left.clone();
    if status.chars().count() + right.chars().count() < width {
        let pad = width - status.chars().count() - right.chars().count();
        status.push_str(&" ".repeat(pad));
        status.push_str(&right);
    } else {
        let take: String = status.chars().take(width).collect();
        status = take;
    }

    term.write(&status)?;
    term.reset_colors()?;
    Ok(())
}

fn draw_message_bar(term: &mut Terminal, view: &View, msg: &str) -> io::Result<()> {
    let y = view.height.saturating_sub(1);
    term.move_to(0, y)?;
    term.clear_line()?;
    if msg.is_empty() {
        return Ok(());
    }
    let bg = Color::Rgb { r: 55, g: 55, b: 55 };
    let fg = Color::White;
    term.set_colors(fg, bg)?;
    let width = view.width as usize;
    let truncated: String = msg.chars().take(width).collect();
    let mut line = format!(" {}", truncated);
    let current_len = line.chars().count();
    if current_len < width {
        line.push_str(&" ".repeat(width - current_len));
    }
    term.write(&line)?;
    term.reset_colors()?;
    Ok(())
}

pub fn draw_help(term: &mut Terminal, view: &View) -> io::Result<()> {
    let lines = [
        "",
        "  YOZ — Terminal matn muharriri",
        "",
        "  NAVIGATSIYA",
        "  Arrow keys        Kursor harakati",
        "  Home / End        Qator boshi / oxiri",
        "  Ctrl+Home/End     Fayl boshi / oxiri",
        "  Ctrl+Left/Right   So'z boshi / oxiri",
        "  Ctrl+Up/Down      Scroll (kursor qimirlamaydi)",
        "  PageUp/Down       Sahifama-sahifa",
        "",
        "  TANLASH",
        "  Shift+Arrow       Belgilab borish",
        "  Shift+Home/End    Qator boshi/oxirigacha",
        "  Ctrl+Shift+L/R    So'zlab tanlash",
        "  Ctrl+A            Hammasini tanlash",
        "  Ikki marta bosish So'zni tanlash",
        "  Esc               Tanlashni bekor qilish",
        "",
        "  TAHRIRLASH",
        "  Ctrl+C            Nusxa olish",
        "  Ctrl+X            Kesib olish",
        "  Ctrl+V            Qo'yish",
        "  Ctrl+Z            Ortga qaytarish (Undo)",
        "  Ctrl+Y            Qayta bajarish (Redo)",
        "  Ctrl+D            Qatorni duplikat",
        "  Ctrl+K            Qator oxirigacha o'chirish",
        "  Ctrl+Backspace    So'zni o'chirish (chapga)",
        "  Ctrl+Delete       So'zni o'chirish (o'ngga)",
        "",
        "  QIDIRISH",
        "  Ctrl+F            Qidirish",
        "  Ctrl+H            Almashtirish",
        "  Ctrl+N / Ctrl+P   Keyingi / Oldingi natija",
        "",
        "  BOSHQA",
        "  Ctrl+S            Saqlash",
        "  Ctrl+E            Encoding tanlash",
        "  Ctrl+L            Qator raqamlari yoq/o'ch",
        "  Ctrl+Q            Chiqish",
        "  F1                Shu yordam oynasi",
        "",
        "  Istalgan tugmani bosib yoping",
        "",
    ];

    let box_w = 50usize;
    let box_h = lines.len();
    let screen_w = view.width as usize;
    let screen_h = view.height as usize;
    let x0 = screen_w.saturating_sub(box_w) / 2;
    let y0 = screen_h.saturating_sub(box_h) / 2;

    let border_fg = Color::Rgb { r: 140, g: 140, b: 140 };
    let bg = Color::Rgb { r: 30, g: 30, b: 30 };
    let text_fg = Color::Rgb { r: 220, g: 220, b: 220 };
    let heading_fg = Color::Rgb { r: 100, g: 180, b: 255 };

    // top border
    term.move_to(x0 as u16, y0 as u16)?;
    term.set_colors(border_fg, bg)?;
    term.write(&format!(
        "\u{256D}{}\u{256E}",
        "\u{2500}".repeat(box_w)
    ))?;

    for (i, line) in lines.iter().enumerate() {
        let y = y0 + 1 + i;
        if y >= screen_h {
            break;
        }
        term.move_to(x0 as u16, y as u16)?;
        term.set_colors(border_fg, bg)?;
        term.write("\u{2502}")?;

        let is_heading = line.starts_with("  ")
            && line.trim().chars().all(|c| c.is_uppercase() || c.is_whitespace());
        let fg = if is_heading { heading_fg } else { text_fg };
        term.set_colors(fg, bg)?;

        let content: String = line.chars().take(box_w).collect();
        let pad = box_w.saturating_sub(content.chars().count());
        term.write(&content)?;
        term.write(&" ".repeat(pad))?;

        term.set_colors(border_fg, bg)?;
        term.write("\u{2502}")?;
    }

    // bottom border
    let yb = y0 + 1 + lines.len();
    if yb < screen_h {
        term.move_to(x0 as u16, yb as u16)?;
        term.set_colors(border_fg, bg)?;
        term.write(&format!(
            "\u{2570}{}\u{256F}",
            "\u{2500}".repeat(box_w)
        ))?;
    }

    term.reset_colors()?;
    term.flush()?;
    Ok(())
}
