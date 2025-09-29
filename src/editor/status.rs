use crate::editor::Editor;
use crate::Result;
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::terminal::ClearType;
use crossterm::{cursor, execute, style, terminal};
use std::io::stdout;

pub fn draw_status_bar(editor: &Editor) -> Result<()> {
    let (width, height) = editor.terminal_size;
    // 状态栏在倒数第二行
    execute!(stdout(), cursor::MoveTo(0, height - 2))?;
    execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;

    // 状态栏
    if let Some(prompt) = &editor.file_save_prompt {
        let input = &editor.file_save_input;
        let msg = format!("{} {}", prompt, input);
        let msg_len = msg.len();
        execute!(
            stdout(),
            SetForegroundColor(Color::Black),
            style::SetBackgroundColor(Color::White),
            style::Print(&msg),
        )?;
        let remaining = width as usize - msg_len;
        if remaining > 0 {
            execute!(stdout(), style::Print(" ".repeat(remaining)))?;
        }
        execute!(stdout(), ResetColor)?;
    } else if editor.exit_confirm_prompt {
        let msg = "文件已修改，是否保存？Y=保存 N=不保存 ^C=取消";
        let msg_len = msg.len();
        execute!(
            stdout(),
            SetForegroundColor(Color::Black),
            style::SetBackgroundColor(Color::White),
            style::Print(msg),
        )?;
        let remaining = width as usize - msg_len;
        if remaining > 0 {
            execute!(stdout(), style::Print(" ".repeat(remaining)))?;
        }
        execute!(stdout(), ResetColor)?;
    } else {
        // 普通状态栏
        let filename = editor
            .buffer
            .filename
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[无文件名]");
        let modified_indicator =
            if editor.buffer.modified && !editor.buffer.modified_lines_set.is_empty() {
                format!(" [已修改 {} 行]", editor.buffer.modified_lines_set.len())
            } else {
                "".to_string()
            };
        let secondary_cursor_indicator = if editor.buffer.cursor_x2.is_some() {
            " [多光标]"
        } else {
            ""
        };
        let mut status = format!(
            " {} - {} 行{}{}",
            filename,
            editor.buffer.lines.len(),
            modified_indicator,
            secondary_cursor_indicator
        );

        if !editor.status_message.is_empty() {
            let left_len = status.len();
            let right_msg = format!("  {}", editor.status_message);
            let space = width as usize - left_len - right_msg.len();
            if space > 0 {
                status.push_str(&" ".repeat(space));
            }
            status.push_str(&right_msg);
        } else {
            let status_len = status.len();
            let remaining = width as usize - status_len;
            if remaining > 0 {
                status.push_str(&" ".repeat(remaining));
            }
        }
        execute!(
            stdout(),
            SetForegroundColor(Color::Black),
            style::SetBackgroundColor(Color::White),
            style::Print(status),
            ResetColor
        )?;
    }
    // 最下方帮助栏始终不被覆盖
    execute!(stdout(), cursor::MoveTo(0, height - 1))?;
    execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
    let help = "^X 退出  ^O 保存  ^G 帮助  ^C 多光标  Alt+方向键 移动多光标";
    execute!(
        stdout(),
        SetForegroundColor(Color::Black),
        style::SetBackgroundColor(Color::White),
        style::Print(help),
    )?;
    let remaining = width as usize - help.len();
    if remaining > 0 {
        execute!(stdout(), style::Print(" ".repeat(remaining)))?;
    }
    execute!(stdout(), ResetColor)?;
    Ok(())
}
