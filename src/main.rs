// RSNano 编辑器主入口点

use rsnano::args::Args;
use rsnano::editor::Editor;
use rsnano::version::read_app_info; // 新增
use rsnano::Result;

fn main() -> Result<()> {
    let args = Args::from_cli()?;
    let app_info = read_app_info();
    let mut editor = Editor::new(args, app_info)?; // 修改签名
    editor.run()
}