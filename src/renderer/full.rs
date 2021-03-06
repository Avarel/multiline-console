use std::{
    convert::TryInto,
    io::{stdout, Stdout, Write},
};

use super::{
    styles::{Footer, Header, Margin, NoStyle},
    Editor, Renderer,
};
use crate::{editor::selection::Cursor, Result};

use crossterm::{
    cursor::*,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use raw_mode::RawModeGuard;

mod raw_mode {
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    use super::Result;

    pub struct RawModeGuard(());

    impl RawModeGuard {
        pub fn acquire() -> Result<RawModeGuard> {
            enable_raw_mode()?;
            Ok(Self(()))
        }
    }

    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            disable_raw_mode().unwrap();
        }
    }
}

pub struct CrosstermRenderer<'b, W, M, H, F> {
    guard: RawModeGuard,
    write: &'b mut W,
    margin: M,
    header: H,
    footer: F,
    draw_state: DrawState,
    max_height: Option<usize>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
/// Contains information about the cursor and the height
/// of the last frame drawn.
struct DrawState {
    altscreen: bool,
    height: usize,
    // Anchor of the box
    anchor: Cursor,
    low: usize,
    high: usize,
    // Box cursor relative to the anchor
    cursor: Cursor,
}

impl<W, M, H, F> Renderer for CrosstermRenderer<'_, W, M, H, F>
where
    W: Write,
    M: Margin<W>,
    H: Header<W>,
    F: Footer<W>,
{
    /// Draw the prompt.
    fn draw(&mut self, data: &Editor) -> Result<()> {
        if self.draw_state.altscreen {
            self.write.queue(MoveTo(0, 0))?;
        } else {
            self.move_to_frame_base()?;
        }

        let (low, high, term_rows) = self.calculate_draw_range(&data);

        if term_rows == 0 {
            return Ok(());
        }

        if data.altscreen && !self.draw_state.altscreen {
            self.write.queue(EnterAlternateScreen)?;
        } else if !data.altscreen && self.draw_state.altscreen {
            self.write.queue(LeaveAlternateScreen)?;
        }

        self.draw_state = DrawState::default();
        self.draw_state.altscreen = data.altscreen;

        self.draw_header(&data)?;
        self.draw_range(&data, low, high, term_rows)?;
        self.draw_footer(&data)?;
        self.write.queue(Clear(ClearType::FromCursorDown))?;

        self.draw_cursor(&data)?;
        self.flush()
    }

    /// Clear the drawn prompt on the screen.
    fn clear_draw(&mut self) -> Result<()> {
        if self.draw_state.altscreen {
            self.write.queue(MoveTo(0, 0))?;
            self.write.queue(Clear(ClearType::All))?;
        } else {
            self.move_to_frame_base()?;
            self.write.queue(Clear(ClearType::FromCursorDown))?;
        }

        self.draw_state = DrawState::default();

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.write.flush()?;
        Ok(())
    }

    fn finish(mut self) -> Result<()> {
        self.clear_draw()?;

        // if self.draw_state.altscreen {
        self.write.queue(LeaveAlternateScreen)?;
        // }

        self.flush()
    }
}

impl<'w, W> DefaultRenderer<'w, W> {
    pub fn render_to(write: &'w mut W) -> Self {
        CrosstermRenderer {
            guard: RawModeGuard::acquire().unwrap(),
            write,
            draw_state: DrawState::default(),
            margin: NoStyle,
            header: NoStyle,
            footer: NoStyle,
            max_height: None,
        }
    }
}

impl<'w, W, M, H, F> CrosstermRenderer<'w, W, M, H, F> {
    pub fn max_height(self, max_height: Option<usize>) -> Self {
        Self { max_height, ..self }
    }
}

// region: Swap constructors
impl<'w, W, M1, H, F> CrosstermRenderer<'w, W, M1, H, F> {
    /// Swap out a margin formatter.
    pub fn margin<M2>(self, margin: M2) -> CrosstermRenderer<'w, W, M2, H, F> {
        CrosstermRenderer {
            guard: self.guard,
            write: self.write,
            draw_state: self.draw_state,
            margin,
            header: self.header,
            footer: self.footer,
            max_height: self.max_height,
        }
    }
}

impl<'w, W, M, H1, F> CrosstermRenderer<'w, W, M, H1, F> {
    /// Swap out a header formatter.
    pub fn header<H2>(self, header: H2) -> CrosstermRenderer<'w, W, M, H2, F> {
        CrosstermRenderer {
            guard: self.guard,
            write: self.write,
            draw_state: self.draw_state,
            margin: self.margin,
            header,
            footer: self.footer,
            max_height: self.max_height,
        }
    }
}

impl<'w, W, M, H, F1> CrosstermRenderer<'w, W, M, H, F1> {
    /// Swap out a footer formatter.
    pub fn footer<F2>(self, footer: F2) -> CrosstermRenderer<'w, W, M, H, F2> {
        CrosstermRenderer {
            guard: self.guard,
            write: self.write,
            draw_state: self.draw_state,
            margin: self.margin,
            header: self.header,
            footer,
            max_height: self.max_height,
        }
    }
}

impl<'w, W, M, H, F> CrosstermRenderer<'w, W, M, H, F>
where
    W: Write,
    M: Margin<W>,
    H: Header<W>,
    F: Footer<W>,
{
    fn calculate_draw_range(&self, data: &Editor) -> (usize, usize, usize) {
        if let Ok((_, rows)) = crossterm::terminal::size() {
            // Rows of the terminal.
            let max_height = if !data.altscreen {
                self.max_height
            } else {
                None
            };
            let term_rows = max_height
                .unwrap_or(usize::MAX)
                .min(rows.try_into().unwrap())
                .saturating_sub(self.header.rows())
                .saturating_sub(self.footer.rows());
            if term_rows == 0 {
                return (0, 0, 0);
            }
            // Rows of the data to draw.
            let data_rows = data.line_count();
            // Current line of the data.
            let line = data.selection.focus.ln;
            if data_rows > term_rows {
                let (low, high) = if line >= self.draw_state.high {
                    (line - term_rows + 1, line + 1)
                } else if line < self.draw_state.low {
                    (line, line + term_rows)
                } else {
                    (self.draw_state.low, self.draw_state.high)
                };
                (low, high.min(data_rows), term_rows)
            } else {
                (0, data.line_count(), term_rows)
            }
        } else {
            (0, data.line_count(), 0)
        }
    }

    // Move to the base of the frame (not the anchor).
    fn move_to_frame_base(&mut self) -> Result<()> {
        let up_offset = self.draw_state.anchor.ln + self.draw_state.cursor.ln;
        self.write.queue(MoveUp(Self::usize_to_u16(up_offset)))?;
        self.write.queue(MoveToColumn(0))?;
        Ok(())
    }

    // Position the cursor right after drawing a frame
    // (assuming no other cursor adjustments made).
    fn draw_cursor(&mut self, data: &Editor) -> Result<()> {
        // Move to the correct row.
        let line = data.selection.focus.ln;
        let frame_height = self.draw_state.height;
        let relative_ln = line - self.draw_state.low;
        let up_offset = frame_height - 1 - self.draw_state.anchor.ln - relative_ln;
        // self.move_cursor_up(up_offset)?;
        self.write.queue(MoveUp(Self::usize_to_u16(up_offset)))?;

        // Move to the correct column.
        let col = data.selection.focus.col.min(data.curr_ln_len());
        let n = self.draw_state.anchor.col + col + 1;
        self.write.queue(MoveToColumn(Self::usize_to_u16(n)))?;

        self.draw_state.cursor.ln = relative_ln;
        self.draw_state.cursor.col = n;

        Ok(())
    }

    fn draw_header(&mut self, data: &Editor) -> Result<()> {
        self.draw_state.height += self.header.rows();
        self.draw_state.anchor.ln += self.header.rows();

        self.cursor_to_left_term_edge()?;
        self.header.draw(self.write, data)?;
        if self.header.rows() > 0 {
            self.write.write(b"\n")?;
        }
        Ok(())
    }

    /// Draw the line given an index.
    /// This method does not move the cursor.
    fn draw_line(&mut self, data: &Editor, line: usize) -> Result<()> {
        self.cursor_to_left_term_edge()?;

        self.margin.draw(self.write, line, data)?;
        if line < data.line_count() {
            data.write_line(line, self.write)?;
        }
        self.write.queue(Clear(ClearType::UntilNewLine))?;

        Ok(())
    }

    fn draw_footer(&mut self, data: &Editor) -> Result<()> {
        self.draw_state.height += self.footer.rows();

        self.cursor_to_left_term_edge()?;
        if self.footer.rows() > 0 {
            self.write.write(b"\n")?;
        }
        // write!(self.write, "{} {} {}", self.draw_state.low, self.draw_state.high, data.cursor.ln)?;
        self.footer.draw(self.write, data)?;
        Ok(())
    }

    fn draw_range(
        &mut self,
        data: &Editor,
        low: usize,
        high: usize,
        term_rows: usize,
    ) -> Result<()> {
        // Print out the contents.
        for i in low..high {
            self.draw_line(&data, i)?;
            if i < high - 1 {
                // The last line should not have any new-line attached to it.
                self.write.write(b"\n")?;
            }
        }

        self.draw_state.anchor.col = self.margin.width();
        self.draw_state.low = low;
        self.draw_state.high = high;
        self.draw_state.height += high - low;
        self.draw_state.cursor.ln = high - low - 1;
        self.draw_state.cursor.col = data.line(high - 1).len();

        if data.altscreen {
            for i in high..low + term_rows {
                self.write.write(b"\n")?;
                self.draw_line(&data, i)?;
            }
            self.draw_state.height += low + term_rows - high;
            self.draw_state.cursor.ln += low + term_rows - high;
        }

        Ok(())
    }

    /// Move the curser to the terminal left margin.
    fn cursor_to_left_term_edge(&mut self) -> Result<()> {
        self.write.queue(MoveToColumn(0))?;
        Ok(())
    }

    #[doc(hidden)]
    fn usize_to_u16(n: usize) -> u16 {
        n.try_into().unwrap_or(std::u16::MAX)
    }
}

pub type DefaultRenderer<'w, W> = CrosstermRenderer<'w, W, NoStyle, NoStyle, NoStyle>;

impl Default for DefaultRenderer<'static, Stdout> {
    fn default() -> Self {
        let out = Box::new(stdout());
        CrosstermRenderer::render_to(Box::leak(out))
    }
}
