use ropey::{Rope, RopeSlice};

pub trait RopeExt {
    fn line_trimmed(&self, line_idx: usize) -> RopeSlice<'_>;
    fn trimmed(&self) -> RopeSlice<'_>;
}

impl RopeExt for Rope {
    fn line_trimmed(&self, line_idx: usize) -> RopeSlice<'_> {
        let line = self.line(line_idx);
        let line_len = line.len_chars();
        if line_len == 0 {
            line
        } else if line.char(line_len - 1) == '\n' {
            line.slice(..line_len - 1)
        } else {
            line.slice(..line_len)
        }
    }

    fn trimmed(&self) -> RopeSlice<'_> {
        let rope = self.slice(..);
        let rope_len = rope.len_chars();
        if rope_len == 0 {
            rope
        } else if rope.char(rope_len - 1) == '\n' {
            rope.slice(..rope_len - 1)
        } else {
            rope.slice(..rope_len)
        }
    }
}