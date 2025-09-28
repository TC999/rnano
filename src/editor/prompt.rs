use crate::editor::Editor;
use crate::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_exit_confirm(editor: &mut Editor, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let init_filename = editor
                .buffer
                .filename
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            editor.file_save_prompt = Some("请输入要保存的文件名（按 ESC 取消）:".to_string());
            editor.file_save_input = init_filename.to_string();
            editor.exit_confirm_prompt = false;
            editor.status_message.clear();
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            editor.should_quit = true;
            editor.exit_confirm_prompt = false;
            editor.status_message.clear();
        }
        KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
            editor.exit_confirm_prompt = false;
            editor.status_message.clear();
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_file_save(editor: &mut Editor, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Enter => {
            let filename = editor.file_save_input.trim();
            if !filename.is_empty() {
                editor.buffer.filename = Some(std::path::PathBuf::from(filename));
                let modified_count = editor.buffer.save()?;
                editor.status_message = format!("已保存，已修改 {} 行", modified_count);
            } else {
                editor.status_message = "文件名不能为空".to_string();
            }
            editor.file_save_prompt = None;
            editor.file_save_input.clear();
        }
        KeyCode::Esc => {
            editor.file_save_prompt = None;
            editor.file_save_input.clear();
            editor.status_message = "已取消保存".to_string();
        }
        KeyCode::Backspace => {
            editor.file_save_input.pop();
        }
        KeyCode::Char(ch) => {
            editor.file_save_input.push(ch);
        }
        _ => {}
    }
    Ok(())
}
