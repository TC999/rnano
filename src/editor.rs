mod input;
mod prompt;
mod status;
mod ui;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::terminal::{ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, event, execute, style, terminal};
use std::io::stdout;

use crate::args::Args;
use crate::buffer::TextBuffer;
use crate::direction::Direction;
use crate::version::AppInfo;
use crate::Result; // 新增

pub struct Editor {
    buffer: TextBuffer,
    terminal_size: (u16, u16),
    show_line_numbers: bool,
    should_quit: bool,
    status_message: String,
    file_save_prompt: Option<String>,
    file_save_input: String,
    exit_confirm_prompt: bool,
    app_info: AppInfo, // 新增
    show_help_page: bool,
    help_page_drawn: bool, // 跟踪帮助页是否已绘制
}

impl Editor {
    pub fn new(args: Args, app_info: AppInfo) -> Result<Self> {
        let buffer = if let Some(file) = &args.file {
            TextBuffer::from_file(file)?
        } else {
            TextBuffer::new()
        };
        let terminal_size = crossterm::terminal::size()?;
        Ok(Self {
            buffer,
            terminal_size,
            show_line_numbers: args.line_numbers,
            should_quit: false,
            status_message: String::new(),
            file_save_prompt: None,
            file_save_input: String::new(),
            exit_confirm_prompt: false,
            app_info, // 新增
            show_help_page: false,
            help_page_drawn: false,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        ui::setup_terminal()?;
        let result = self.main_loop();
        ui::restore_terminal()?;
        result
    }

    fn main_loop(&mut self) -> Result<()> {
        loop {
            self.refresh_screen()?;
            if self.should_quit {
                break;
            }
            if crossterm::event::poll(std::time::Duration::from_millis(50))? {
                if let crossterm::event::Event::Key(key_event) = crossterm::event::read()? {
                    if key_event.kind == crossterm::event::KeyEventKind::Press {
                        self.process_key(key_event)?;
                    }
                }
            }
            let new_size = crossterm::terminal::size()?;
            if new_size != self.terminal_size {
                self.terminal_size = new_size;
                // 如果正在显示帮助页且终端大小改变，需要重新绘制
                if self.show_help_page {
                    self.help_page_drawn = false;
                }
            }
        }
        Ok(())
    }
  
    fn process_key(&mut self, key_event: KeyEvent) -> Result<()> {
        // 退出确认模式
        if self.exit_confirm_prompt {
            match key_event.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let init_filename = self
                        .buffer
                        .filename
                        .as_ref()
                        .and_then(|p| p.to_str())
                        .unwrap_or("");
                    self.file_save_prompt =
                        Some("请输入要保存的文件名（按 ESC 取消）:".to_string());
                    self.file_save_input = init_filename.to_string();
                    self.exit_confirm_prompt = false;
                    self.status_message.clear();
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.should_quit = true;
                    self.exit_confirm_prompt = false;
                    self.status_message.clear();
                }
                KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.exit_confirm_prompt = false;
                    self.status_message.clear();
                }
                _ => {}
            }
            return Ok(());
        }

        // 文件名输入模式
        if self.file_save_prompt.is_some() {
            match key_event.code {
                KeyCode::Enter => {
                    let filename = self.file_save_input.trim();
                    if !filename.is_empty() {
                        self.buffer.filename = Some(std::path::PathBuf::from(filename));
                        let modified_count = self.buffer.save()?; // 获取实际修改行数
                        self.status_message = format!("已保存，已修改 {} 行", modified_count);
                    } else {
                        self.status_message = "文件名不能为空".to_string();
                    }
                    self.file_save_prompt = None;
                    self.file_save_input.clear();
                }
                KeyCode::Esc => {
                    self.file_save_prompt = None;
                    self.file_save_input.clear();
                    self.status_message = "已取消保存".to_string();
                }
                KeyCode::Backspace => {
                    self.file_save_input.pop();
                }
                KeyCode::Char(ch) => {
                    self.file_save_input.push(ch);
                }
                _ => {}
            }
            return Ok(());
        }

        // 显示帮助页面
        // 如果正在显示帮助页面，任意按键关闭帮助页面
        if self.show_help_page {
            match key_event.code {
                KeyCode::Esc | KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace => {
                    self.show_help_page = false;
                    self.help_page_drawn = false; // 重置帮助页绘制状态
                    self.status_message.clear();
                }
                _ => {}
            }
            return Ok(());
        }

        // Ctrl+G 打开帮助页面
        if let KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } = key_event
        {
            self.show_help_page = true;
            self.help_page_drawn = false; // 标记需要重新绘制帮助页
            self.status_message = "按任意键返回编辑器".to_string();
            return Ok(());
        }

        match key_event {
            KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                if self.buffer.modified {
                    self.exit_confirm_prompt = true;
                    self.status_message =
                        "文件已修改，是否保存？Y=保存 N=不保存 ^C=取消".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            KeyEvent {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                let init_filename = self
                    .buffer
                    .filename
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("");
                self.file_save_prompt = Some("请输入要保存的文件名（按 ESC 取消）:".to_string());
                self.file_save_input = init_filename.to_string();
            }
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer.toggle_secondary_cursor();
                self.status_message = if self.buffer.cursor_x2.is_some() {
                    "多光标已启用".to_string()
                } else {
                    "多光标已关闭".to_string()
                };
            }
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer
                    .move_cursor(Direction::Up, self.terminal_size, true);
            }
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer
                    .move_cursor(Direction::Down, self.terminal_size, true);
            }
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer
                    .move_cursor(Direction::Left, self.terminal_size, true);
            }
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer
                    .move_cursor(Direction::Right, self.terminal_size, true);
            }
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer
                    .move_cursor(Direction::Up, self.terminal_size, false);
            }
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer
                    .move_cursor(Direction::Down, self.terminal_size, false);
            }
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer
                    .move_cursor(Direction::Left, self.terminal_size, false);
            }
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer
                    .move_cursor(Direction::Right, self.terminal_size, false);
            }
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.insert_newline();
            }
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.delete_char();
            }
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::CONTROL,
                ..
            } if self.buffer.cursor_x2.is_some() && self.buffer.cursor_y2.is_some() => {
                self.buffer.insert_char_at_both_cursors(ch);
            }
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers,
                ..
            } if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
                self.buffer.insert_char(ch);
            }
            _ => {}
        }
        Ok(())
    }

    fn refresh_screen(&mut self) -> Result<()> {
        use crossterm::style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor};
        use crossterm::terminal::ClearType;
        use crossterm::{cursor, execute, style, terminal};
        use std::io::stdout;

        // 顶部信息栏
        execute!(stdout(), cursor::MoveTo(0, 0))?;
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;

        let filename = self
            .buffer
            .filename
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("新缓冲区");

        let info_bar = format!(
            "{} v{}    文件: {}",
            self.app_info.name, self.app_info.version, filename
        );

        execute!(
            stdout(),
            SetForegroundColor(Color::White),
            style::SetBackgroundColor(Color::Blue),
            style::Print(&info_bar),
            ResetColor
        )?;

        // 帮助页面处理
        if self.show_help_page {
            // 只有在帮助页面还未绘制时才绘制，避免频闪
            if !self.help_page_drawn {
                self.draw_help_page()?;
                self.help_page_drawn = true;
            }
            return Ok(());
        }

        // 编辑器区域
        let (width, height) = self.terminal_size;
        let editor_height = height - 3; // 顶部信息栏占1行

        execute!(stdout(), cursor::MoveTo(0, 1))?;
        for screen_row in 0..editor_height {
            let file_row = screen_row as usize + self.buffer.offset_y;
            execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
            if file_row < self.buffer.lines.len() {
                let line = &self.buffer.lines[file_row];
                let line_number_width = if self.show_line_numbers { 4 } else { 0 };
                if self.show_line_numbers {
                    execute!(
                        stdout(),
                        SetForegroundColor(Color::Yellow),
                        style::Print(format!("{:3} ", file_row + 1)),
                        ResetColor
                    )?;
                }
                let display_width = width as usize - line_number_width;
                let start = self.buffer.offset_x.min(line.chars().count());
                let end = (start + display_width).min(line.chars().count());
                for (i, ch) in line.chars().enumerate().skip(start).take(end - start) {
                    if i == self.buffer.cursor_x && file_row == self.buffer.cursor_y {
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
                if self.buffer.cursor_y == file_row
                    && self.buffer.cursor_x == line.chars().count()
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
        self.draw_status_bar()?;
        Ok(())
    }

    fn draw_status_bar(&self) -> Result<()> {
        let (width, height) = self.terminal_size;
        // 状态栏在倒数第二行
        execute!(stdout(), cursor::MoveTo(0, height - 2))?;
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;

        // 状态栏
        if let Some(prompt) = &self.file_save_prompt {
            let input = &self.file_save_input;
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
        } else if self.exit_confirm_prompt {
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
            // 普通状态栏：文件名、行数、修改状态、多光标、操作提示
            let filename = self
                .buffer
                .filename
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("[无文件名]");
            let modified_indicator =
                if self.buffer.modified && !self.buffer.modified_lines_set.is_empty() {
                    format!(" [已修改 {} 行]", self.buffer.modified_lines_set.len())
                } else {
                    "".to_string()
                };
            let secondary_cursor_indicator = if self.buffer.cursor_x2.is_some() {
                " [多光标]"
            } else {
                ""
            };
            let mut status = format!(
                " {} - {} 行{}{}",
                filename,
                self.buffer.lines.len(),
                modified_indicator,
                secondary_cursor_indicator
            );

            // 状态栏右侧显示操作状态提示（如保存成功、失败等）
            if !self.status_message.is_empty() {
                // 状态栏右侧显示
                let left_len = status.len();
                let right_msg = format!("  {}", self.status_message);
                let space = width as usize - left_len - right_msg.len();
                if space > 0 {
                    status.push_str(&" ".repeat(space));
                }
                status.push_str(&right_msg);
            } else {
                // 补齐到整个状态栏宽度
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
        let help = "^X 退出  ^O 保存  ^C 多光标  Alt+方向键 移动多光标";
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

    fn draw_help_page(&self) -> Result<()> {
        let (_width, height) = self.terminal_size;
        execute!(
            stdout(),
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )?;

        let help_lines = [
            "RSNano 帮助页面",
            "",
            "^X 退出编辑器",
            "^O 保存文件",
            "^C 多光标模式开/关",
            "Alt+方向键 移动多光标",
            "^G 打开帮助页面",
            "",
            "按任意键返回编辑器",
        ];

        for (i, line) in help_lines.iter().enumerate() {
            if i < height as usize {
                execute!(stdout(), cursor::MoveTo(0, i as u16), style::Print(line))?;
            }
        }
        Ok(())
    }
}
