use crate::editor::Editor;
use crate::Result;
use crossterm::style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::ClearType;
use crossterm::{cursor, execute, style, terminal};
use std::io::stdout;

pub fn setup_terminal() -> Result<()> {
    terminal::enable_raw_mode()?;
    execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
    Ok(())
}

pub fn restore_terminal() -> Result<()> {
    execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

pub fn refresh_screen(editor: &mut Editor) -> Result<()> {
    // 顶部信息栏
    execute!(stdout(), cursor::MoveTo(0, 0))?;
    execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
    let filename = editor
        .buffer
        .filename
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("新缓冲区");
    let info_bar = format!(
        "{} v{}    文件: {}",
        editor.app_info.name, editor.app_info.version, filename
    );
    execute!(
        stdout(),
        SetForegroundColor(Color::White),
        style::SetBackgroundColor(Color::Blue),
        style::Print(&info_bar),
        ResetColor
    )?;

    // 编辑器区域
    let (width, height) = editor.terminal_size;
    let editor_height = height - 3;
    execute!(stdout(), cursor::MoveTo(0, 1))?;
    for screen_row in 0..editor_height {
        let file_row = screen_row as usize + editor.buffer.offset_y;
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
        if file_row < editor.buffer.lines.len() {
            let line = &editor.buffer.lines[file_row];
            let line_number_width = if editor.show_line_numbers { 4 } else { 0 };
            if editor.show_line_numbers {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Yellow),
                    style::Print(format!("{:3} ", file_row + 1)),
                    ResetColor
                )?;
            }
            let display_width = width as usize - line_number_width;
            let start = editor.buffer.offset_x.min(line.chars().count());
            let end = (start + display_width).min(line.chars().count());
            for (i, ch) in line.chars().enumerate().skip(start).take(end - start) {
                if i == editor.buffer.cursor_x && file_row == editor.buffer.cursor_y {
                    execute!(
                        stdout(),
                        SetBackgroundColor(Color::Yellow),
                        SetForegroundColor(Color::Black),
                        style::Print(ch),
                        ResetColor
                    )?;
                } else {
                    execute!(stdout(), style::Print(ch))?;
                }
            }
            if editor.buffer.cursor_y == file_row
                && editor.buffer.cursor_x == line.chars().count()
                && end == line.chars().count()
            {
                execute!(
                    stdout(),
                    SetBackgroundColor(Color::Yellow),
                    SetForegroundColor(Color::Black),
                    style::Print("▏"),
                    ResetColor
                )?;
            }
        }
        execute!(stdout(), cursor::MoveToNextLine(1))?;
    }
    super::status::draw_status_bar(editor)?;
    Ok(())
}
