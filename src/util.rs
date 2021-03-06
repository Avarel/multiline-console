use ropey::RopeSlice;
 
pub(crate) fn trimmed(rope: RopeSlice) -> RopeSlice {
    let rope_len = rope.len_chars();
    if rope_len == 0 {
        rope
    } else if rope.char(rope_len - 1) == '\n' {
        rope.slice(..rope_len - 1)
    } else {
        rope
    }
}
