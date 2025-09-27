use crossterm::{cursor, event, style, terminal, execute};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, KeyEventKind};
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::terminal::{ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use std::io::{stdout, Write};

use crate::buffer::TextBuffer;
use crate::direction::Direction;
use crate::args::Args;
use crate::Result;

/// 编辑器主结构
pub struct Editor {
    buffer: TextBuffer,
    terminal_size: (u16, u16),
    show_line_numbers: bool,
    should_quit: bool,
    status_message: String,
    file_save_prompt: Option<String>, // 新增：保存文件名弹窗
    file_save_input: String,          // 新增：输入框内容
}

impl Editor {
    /// 创建一个新的编辑器实例
    pub fn new(args: Args) -> Result<Self> {
        let buffer = if let Some(file) = &args.file {
            TextBuffer::from_file(file)?
        } else {
            TextBuffer::new()
        };

        let terminal_size = terminal::size()?;
        
        Ok(Self {
            buffer,
            terminal_size,
            show_line_numbers: args.line_numbers,
            should_quit: false,
            status_message: String::new(),
            file_save_prompt: None,
            file_save_input: String::new(),
        })
    }

    /// 运行编辑器主循环
    pub fn run(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;

        let result = self.main_loop();

        execute!(stdout(), LeaveAlternateScreen, cursor::Show)?;
        terminal::disable_raw_mode()?;

        result
    }

    /// 编辑器主循环，处理事件和刷新屏幕
    fn main_loop(&mut self) -> Result<()> {
        loop {
            self.refresh_screen()?;

            if self.should_quit {
                break;
            }

            // 使用事件轮询而非阻塞读取
            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key_event) = event::read()? {
                    // 只处理 KeyEventKind::Press，彻底解决重复插入/删除问题
                    if key_event.kind == KeyEventKind::Press {
                        self.process_key(key_event)?;
                    }
                }
            }

            // Update terminal size if changed
            let new_size = terminal::size()?;
            if new_size != self.terminal_size {
                self.terminal_size = new_size;
            }
        }
        Ok(())
    }

    /// 处理按键事件
    fn process_key(&mut self, key_event: KeyEvent) -> Result<()> {
        // 保存文件名输入弹窗模式
        if self.file_save_prompt.is_some() {
            match key_event.code {
                KeyCode::Enter => {
                    // 确认输入并保存
                    let filename = self.file_save_input.trim();
                    if !filename.is_empty() {
                        self.buffer.filename = Some(std::path::PathBuf::from(filename));
                        if self.buffer.save()? {
                            self.status_message = "文件已保存".to_string();
                        } else {
                            self.status_message = "保存失败".to_string();
                        }
                    } else {
                        self.status_message = "文件名不能为空".to_string();
                    }
                    self.file_save_prompt = None;
                    self.file_save_input.clear();
                }
                KeyCode::Esc => {
                    // 取消保存
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
            return Ok(()); // 输入模式下不处理其他按键
        }

        match key_event {
            KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                if self.buffer.modified && self.status_message.contains("File modified") {
                    self.should_quit = true;
                } else if self.buffer.modified {
                    self.status_message = "文件已修改。再次按 Ctrl+X 退出不保存，或按 Ctrl+O 保存".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            KeyEvent {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                // 总是弹出文件名输入弹窗，初始内容为当前文件名或空
                let init_filename = self.buffer.filename
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("");
                self.file_save_prompt = Some("请输入要保存的文件名:".to_string());
                self.file_save_input = init_filename.to_string();
            }
            // 切换第二个光标显示/隐藏的快捷键
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
            // 使用Alt+方向键移动第二个光标
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer.move_cursor(Direction::Up, self.terminal_size, true);
            }
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer.move_cursor(Direction::Down, self.terminal_size, true);
            }
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer.move_cursor(Direction::Left, self.terminal_size, true);
            }
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer.move_cursor(Direction::Right, self.terminal_size, true);
            }
            // 主光标移动
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.move_cursor(Direction::Up, self.terminal_size, false);
            }
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.move_cursor(Direction::Down, self.terminal_size, false);
            }
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.move_cursor(Direction::Left, self.terminal_size, false);
            }
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.move_cursor(Direction::Right, self.terminal_size, false);
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
            // 使用Ctrl+字符在两个光标位置同时插入
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::CONTROL,
                ..
            } if self.buffer.cursor_x2.is_some() && self.buffer.cursor_y2.is_some() => {
                self.buffer.insert_char_at_both_cursors(ch);
            }
            // 普通字符输入
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

    /// 屏幕刷新，主光标高亮，支持中文
    fn refresh_screen(&mut self) -> Result<()> {
        use std::io::stdout;
        use crossterm::{cursor, style, terminal, execute};
        use crossterm::style::{Color, ResetColor, SetForegroundColor, SetBackgroundColor};
        use crossterm::terminal::ClearType;
    
        execute!(stdout(), cursor::MoveTo(0, 0))?;
        let (width, height) = self.terminal_size;
        let editor_height = height - 2;
    
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
                        execute!(
                            stdout(),
                            style::Print(ch)
                        )?;
                    }
                }
                if self.buffer.cursor_y == file_row 
                    && self.buffer.cursor_x == line.chars().count()
                    && end == line.chars().count() {
                    execute!(
                        stdout(),
                        SetBackgroundColor(Color::Yellow),
                        SetForegroundColor(Color::Black),
                        style::Print("▏"),
                        ResetColor
                    )?;
                }
            } else if file_row == self.buffer.lines.len() && screen_row == 0 {
                let welcome = "RSNano - Rust实现的nano文本编辑器";
                if welcome.len() < width as usize {
                    let padding = (width as usize - welcome.len()) / 2;
                    execute!(
                        stdout(),
                        cursor::MoveTo(padding as u16, screen_row),
                        SetForegroundColor(Color::Blue),
                        style::Print(welcome),
                        ResetColor
                    )?;
                }
            }
            execute!(stdout(), cursor::MoveToNextLine(1))?;
        }
    
        // 菜单栏和帮助栏
        self.draw_status_bar()?;
        self.draw_help_bar()?;

        // 如果弹窗模式，绘制弹窗
        if let Some(prompt) = &self.file_save_prompt {
            let (width, height) = self.terminal_size;
            let popup_y = height / 2;
            let popup_x = (width as usize / 2).saturating_sub(20) as u16;
            let input = &self.file_save_input;
            let popup = format!("{} {}", prompt, input);
            // 清理弹窗区，避免残留
            execute!(
                stdout(),
                cursor::MoveTo(popup_x, popup_y),
                terminal::Clear(ClearType::CurrentLine),
                SetBackgroundColor(Color::White),
                SetForegroundColor(Color::Black),
                style::Print(&popup),
                ResetColor
            )?;
        }
    
        // 主光标定位
        let line_number_width = if self.show_line_numbers { 4 } else { 0 };
        let main_screen_x = (self.buffer.cursor_x - self.buffer.offset_x + line_number_width) as u16;
        let main_screen_y = (self.buffer.cursor_y - self.buffer.offset_y) as u16;
        execute!(stdout(), cursor::MoveTo(main_screen_x, main_screen_y))?;
        stdout().flush()?;
        Ok(())
    }

    /// 绘制状态栏
    fn draw_status_bar(&self) -> Result<()> {
        let (width, height) = self.terminal_size;
        execute!(stdout(), cursor::MoveTo(0, height - 2))?;
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
        
        let filename = self.buffer.filename
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[无文件名]");
        
        let modified_indicator = if self.buffer.modified { " [已修改]" } else { "" };
        let secondary_cursor_indicator = if self.buffer.cursor_x2.is_some() { " [多光标]" } else { "" };
        let status = format!(" {} - {} 行{}{}", filename, self.buffer.lines.len(), modified_indicator, secondary_cursor_indicator);
        let status_len = status.len();
        
        execute!(
            stdout(),
            SetForegroundColor(Color::Black),
            style::SetBackgroundColor(Color::White),
            style::Print(status),
        )?;
        
        // 补齐状态栏
        let remaining = width as usize - status_len;
        if remaining > 0 {
            execute!(stdout(), style::Print(" ".repeat(remaining)))?;
        }
        
        execute!(stdout(), ResetColor)?;
        
        // 显示状态消息
        if !self.status_message.is_empty() {
            execute!(stdout(), cursor::MoveToNextLine(1))?;
            execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
            execute!(
                stdout(),
                SetForegroundColor(Color::Red),
                style::Print(&self.status_message),
                ResetColor
            )?;
        }
        
        Ok(())
    }

    /// 绘制帮助栏
    fn draw_help_bar(&self) -> Result<()> {
        let (width, height) = self.terminal_size;
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
}