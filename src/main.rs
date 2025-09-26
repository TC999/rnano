// RSNano 编辑器主入口点

use rsnano::args::Args;
use rsnano::editor::Editor;
use rsnano::Result;

fn main() -> Result<()> {
    let args = Args::from_cli()?;
    let mut editor = Editor::new(args)?;
    editor.run()
}