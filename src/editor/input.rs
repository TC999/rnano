use crate::direction::Direction;
use crate::editor::Editor;
use crate::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn process_key(editor: &mut Editor, key_event: KeyEvent) -> Result<()> {
    // 退出确认模式
    if editor.exit_confirm_prompt {
        return super::prompt::handle_exit_confirm(editor, key_event);
    }
    // 文件名输入模式
    if editor.file_save_prompt.is_some() {
        return super::prompt::handle_file_save(editor, key_event);
    }

    match key_event {
        KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            if editor.buffer.modified {
                editor.exit_confirm_prompt = true;
                editor.status_message = "文件已修改，是否保存？Y=保存 N=不保存 ^C=取消".to_string();
            } else {
                editor.should_quit = true;
            }
        }
        KeyEvent {
            code: KeyCode::Char('o'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            let init_filename = editor
                .buffer
                .filename
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            editor.file_save_prompt = Some("请输入要保存的文件名（按 ESC 取消）:".to_string());
            editor.file_save_input = init_filename.to_string();
        }
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::ALT,
            ..
        } => {
            editor.buffer.toggle_secondary_cursor();
            editor.status_message = if editor.buffer.cursor_x2.is_some() {
                "多光标已启用".to_string()
            } else {
                "多光标已关闭".to_string()
            };
        }
        // Ctrl+G 打开帮助页面
        KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            editor.show_help_page = true;
            editor.help_page_drawn = false; // 确保下次会重新绘制帮助页面
            editor.status_message = "按任意键返回编辑器".to_string();
            return Ok(());
        }
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::ALT,
            ..
        } => {
            editor
                .buffer
                .move_cursor(Direction::Up, editor.terminal_size, true);
        }
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::ALT,
            ..
        } => {
            editor
                .buffer
                .move_cursor(Direction::Down, editor.terminal_size, true);
        }
        KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::ALT,
            ..
        } => {
            editor
                .buffer
                .move_cursor(Direction::Left, editor.terminal_size, true);
        }
        KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::ALT,
            ..
        } => {
            editor
                .buffer
                .move_cursor(Direction::Right, editor.terminal_size, true);
        }
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            editor
                .buffer
                .move_cursor(Direction::Up, editor.terminal_size, false);
        }
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            editor
                .buffer
                .move_cursor(Direction::Down, editor.terminal_size, false);
        }
        KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            editor
                .buffer
                .move_cursor(Direction::Left, editor.terminal_size, false);
        }
        KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            editor
                .buffer
                .move_cursor(Direction::Right, editor.terminal_size, false);
        }
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            editor.buffer.insert_newline();
        }
        KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            editor.buffer.delete_char();
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::CONTROL,
            ..
        } if editor.buffer.cursor_x2.is_some() && editor.buffer.cursor_y2.is_some() => {
            editor.buffer.insert_char_at_both_cursors(ch);
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        } if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
            editor.buffer.insert_char(ch);
        }
        _ => {}
    }
    Ok(())
}
