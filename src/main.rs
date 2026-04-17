mod buffer;
mod editor;
mod encoding;
mod history;
mod selection;
mod terminal;
mod ui;

use std::env;
use std::process;

use editor::Editor;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_path = args.get(1).cloned();

    let mut editor = match Editor::new(file_path) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("yoz: ishga tushirishda xato: {err}");
            process::exit(1);
        }
    };

    if let Err(err) = editor.run() {
        eprintln!("yoz: xato: {err}");
        process::exit(1);
    }
}
