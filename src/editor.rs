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
        match key_event {
            KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                if self.buffer.modified && self.status_message.contains("File modified") {
                    self.should_quit = true;
                } else if self.buffer.modified {
                    self.status_message = "File modified. Press Ctrl+X again to exit without saving, or Ctrl+O to save".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            KeyEvent {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                if self.buffer.save()? {
                    self.status_message = "File saved".to_string();
                } else {
                    self.status_message = "No filename specified".to_string();
                }
            }
            // 切换第二个光标显示/隐藏的快捷键
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::ALT,
                ..
            } => {
                self.buffer.toggle_secondary_cursor();
                self.status_message = if self.buffer.cursor_x2.is_some() { 
                    "Secondary cursor enabled".to_string() 
                } else { 
                    "Secondary cursor disabled".to_string() 
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

    /// 刷新屏幕，绘制编辑器内容和UI元素
    fn refresh_screen(&mut self) -> Result<()> {
        execute!(stdout(), cursor::MoveTo(0, 0))?;
        
        let (width, height) = self.terminal_size;
        let editor_height = height - 2; // Reserve space for status and help

        // Draw editor content
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
                let start = self.buffer.offset_x.min(line.len());
                let end = (start + display_width).min(line.len());
                
                if start < line.len() {
                    print!("{}", &line[start..end]);
                }
            } else if file_row == self.buffer.lines.len() && screen_row == 0 {
                // Show welcome message for empty buffer
                let welcome = "RSNano - Rust implementation of nano text editor";
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

        // Draw status bar
        self.draw_status_bar()?;
        
        // Draw help bar
        self.draw_help_bar()?;

        // 先保存主光标位置
        let line_number_width = if self.show_line_numbers { 4 } else { 0 };
        let main_screen_x = (self.buffer.cursor_x - self.buffer.offset_x + line_number_width) as u16;
        let main_screen_y = (self.buffer.cursor_y - self.buffer.offset_y) as u16;
        
        // 如果有第二个光标，先显示它
        if let (Some(x2), Some(y2)) = (self.buffer.cursor_x2, self.buffer.cursor_y2) {
            let screen_x2 = (x2 - self.buffer.offset_x + line_number_width) as u16;
            let screen_y2 = (y2 - self.buffer.offset_y) as u16;
            
            // 确保第二个光标在可视区域内
            if screen_y2 < editor_height {
                // 显示第二个光标（使用不同颜色区分）
                execute!(stdout(), cursor::SavePosition)?;
                execute!(stdout(), cursor::MoveTo(screen_x2, screen_y2))?;
                execute!(stdout(), SetForegroundColor(Color::Green))?;
                execute!(stdout(), cursor::Show)?;
                execute!(stdout(), cursor::RestorePosition)?;
            }
        }
        
        // 最后定位主光标
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
            .unwrap_or("[No Name]");
        
        let modified_indicator = if self.buffer.modified { " [Modified]" } else { "" };
        let secondary_cursor_indicator = if self.buffer.cursor_x2.is_some() { " [Multi-cursor]" } else { "" };
        let status = format!(" {} - {} lines{}{}", filename, self.buffer.lines.len(), modified_indicator, secondary_cursor_indicator);
        let status_len = status.len();
        
        execute!(
            stdout(),
            SetForegroundColor(Color::Black),
            style::SetBackgroundColor(Color::White),
            style::Print(status),
        )?;
        
        // Fill rest of status bar
        let remaining = width as usize - status_len;
        if remaining > 0 {
            execute!(stdout(), style::Print(" ".repeat(remaining)))?;
        }
        
        execute!(stdout(), ResetColor)?;
        
        // Show status message if any
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
        
        let help = "^X Exit  ^O Save  ^C Toggle cursor  Alt+Arrows Move 2nd cursor";
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