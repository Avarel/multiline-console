use minime::{
    editor::{keybindings::NormalKeybinding, Editor},
    renderer::{
        full::CrosstermRenderer,
        styles::fancy::{FancyFooter, FancyGutter, FancyHeader},
    },
    Result,
};

fn main() -> Result<()> {
    // Redirect our output to stdout (default).
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();

    let renderer = CrosstermRenderer::render_to(&mut lock)
        .max_height(Some(10))
        .header(FancyHeader {
            message: "Type something :)",
        })
        .margin(FancyGutter)
        .footer(FancyFooter);

    // Print out some prompt using styling options.
    let mut term = Editor::default();
    term.read(NormalKeybinding, renderer)?;
    dbg!(term.contents());
    Ok(())
}
