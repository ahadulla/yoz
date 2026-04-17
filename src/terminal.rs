use std::io::{self, Stdout, Write, stdout};

use crossterm::{
    cursor, execute, queue,
    event::{DisableMouseCapture, EnableMouseCapture},
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    },
};

pub use crossterm::style::Color;

pub struct Terminal {
    out: Stdout,
}

impl Terminal {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Self { out })
    }

    pub fn begin_sync(&mut self) -> io::Result<()> {
        queue!(self.out, BeginSynchronizedUpdate)?;
        Ok(())
    }

    pub fn end_sync(&mut self) -> io::Result<()> {
        queue!(self.out, EndSynchronizedUpdate)?;
        Ok(())
    }

    pub fn size() -> io::Result<(u16, u16)> {
        terminal::size()
    }

    pub fn move_to(&mut self, x: u16, y: u16) -> io::Result<()> {
        queue!(self.out, cursor::MoveTo(x, y))?;
        Ok(())
    }

    pub fn clear_line(&mut self) -> io::Result<()> {
        queue!(self.out, Clear(ClearType::CurrentLine))?;
        Ok(())
    }

    pub fn write(&mut self, s: &str) -> io::Result<()> {
        queue!(self.out, Print(s))?;
        Ok(())
    }

    pub fn set_colors(&mut self, fg: Color, bg: Color) -> io::Result<()> {
        queue!(self.out, SetForegroundColor(fg), SetBackgroundColor(bg))?;
        Ok(())
    }

    pub fn reset_colors(&mut self) -> io::Result<()> {
        queue!(self.out, ResetColor)?;
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(
            self.out,
            DisableMouseCapture,
            ResetColor,
            cursor::Show,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}
