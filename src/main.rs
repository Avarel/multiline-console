use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Write},
};

use clap::{App, Arg};
use mini_me::{
    editor::{keybindings::NormalKeybinding, Editor},
    renderer::{
        full::CrosstermRenderer,
        styles::fancy::{FancyFooter, FancyGutter},
    },
    Result,
};

fn main() -> Result<()> {
    let matches = App::new("Mini-Me")
        .version("0.1")
        .author("Avarel <avarelpm@gmail.com>")
        .about("Miniaturized text editor")
        .arg(
            Arg::with_name("height")
                .long("height")
                .value_name("INTEGER")
                .help("Sets the window height of the editor")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FILE")
                .help("Sets the input file to use")
                .takes_value(true),
        )
        .get_matches();

    let max_height = matches.value_of("height").and_then(|s| s.parse().ok());

    let file_path = matches.value_of("FILE");

    let file = file_path.and_then(|path| OpenOptions::new().read(true).open(path).ok());

    let stderr = std::io::stderr();
    let mut lock = stderr.lock();

    let renderer = CrosstermRenderer::render_to(&mut lock)
        .max_height(max_height)
        .margin(FancyGutter)
        .footer(FancyFooter);

    let mut term = Editor::with_renderer(renderer);

    if let Some(file) = file {
        term.set_contents(BufReader::new(file))?;
        term.cursor().move_to_bottom();
        term.cursor().move_to_line_end();
    }

    let contents = term.read(NormalKeybinding)?;

    if let Some(file) = file_path.and_then(|path| {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .ok()
    }) {
        let mut writer = BufWriter::new(file);
        writer.write_all(contents.as_bytes())?;
    } else {
        std::io::stdout().lock().write_all(contents.as_bytes())?;
    }

    Ok(())
}