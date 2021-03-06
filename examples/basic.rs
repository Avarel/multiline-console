use minime::{editor::keybindings::NormalKeybinding, editor::Editor, Result, renderer::full::DefaultRenderer};

// Basic bare bones example.
//
// Sample output:
//
//      Write something cool!
//      hello there
//      how are you?
//      [examples\basic.rs:14] term.read_multiline() = Ok(
//          "hello there\nhow are you?",
//      )

fn main() -> Result<()> {
    println!("Write something cool!");
    // Build the prompt.
    let mut term = Editor::default();
    term.read(NormalKeybinding, DefaultRenderer::default())?;
    dbg!(term.contents());
    Ok(())
}
