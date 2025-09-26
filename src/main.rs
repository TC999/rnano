use clap::Parser;
use crossterm::{
    cursor::{self, Show, Hide},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{self, Color, ResetColor, SetForegroundColor},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Write};
use std::fs;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// File to edit
    file: Option<PathBuf>,
    
    /// Show line numbers
    #[arg(short, long)]
    line_numbers: bool,
}

#[derive(Clone)]
pub struct TextBuffer {
    lines: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    offset_x: usize,
    offset_y: usize,
    modified: bool,
    filename: Option<PathBuf>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            offset_x: 0,
            offset_y: 0,
            modified: false,
            filename: None,
        }
    }

    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let contents = fs::read_to_string(path).unwrap_or_default();
        let lines = if contents.is_empty() {
            vec![String::new()]
        } else {
            contents.lines().map(|s| s.to_string()).collect()
        };

        Ok(Self {
            lines,
            cursor_x: 0,
            cursor_y: 0,
            offset_x: 0,
            offset_y: 0,
            modified: false,
            filename: Some(path.clone()),
        })
    }

    pub fn current_line(&self) -> &String {
        &self.lines[self.cursor_y]
    }

    pub fn current_line_mut(&mut self) -> &mut String {
        &mut self.lines[self.cursor_y]
    }

    pub fn insert_char(&mut self, ch: char) {
        // 先保存cursor_x的值，避免借用冲突
        let cursor_x = self.cursor_x;
        let line = self.current_line_mut();
        
        // 检查当前位置是否已有相同字符（防止重复）
        if cursor_x < line.len() && line.chars().nth(cursor_x).unwrap_or('\0') == ch {
            return; // 如果字符相同且位置相同，不执行插入
        }
        
        line.insert(cursor_x, ch);
        self.cursor_x += 1;
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        let current_line = self.current_line().clone();
        let (left, right) = current_line.split_at(self.cursor_x);
        
        self.lines[self.cursor_y] = left.to_string();
        self.lines.insert(self.cursor_y + 1, right.to_string());
        
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.modified = true;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_x > 0 {
            // 先保存cursor_x的值，避免借用冲突
            let cursor_x = self.cursor_x;
            let line = self.current_line_mut();
            
            // 确保不会重复删除
            if cursor_x <= line.len() {
                line.remove(cursor_x - 1);
                self.cursor_x -= 1;
                self.modified = true;
            }
        } else if self.cursor_y > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.current_line().len();
            self.current_line_mut().push_str(&current_line);
            self.modified = true;
        }
    }

    pub fn move_cursor(&mut self, direction: Direction, terminal_size: (u16, u16)) {
        match direction {
            Direction::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = self.cursor_x.min(self.current_line().len());
                }
            }
            Direction::Down => {
                if self.cursor_y < self.lines.len() - 1 {
                    self.cursor_y += 1;
                    self.cursor_x = self.cursor_x.min(self.current_line().len());
                }
            }
            Direction::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = self.current_line().len();
                }
            }
            Direction::Right => {
                if self.cursor_x < self.current_line().len() {
                    self.cursor_x += 1;
                } else if self.cursor_y < self.lines.len() - 1 {
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                }
            }
        }
        
        // Adjust scroll offset if cursor goes off screen
        let (_, height) = terminal_size;
        let editor_height = height as usize - 2; // Reserve space for status bar and help
        
        if self.cursor_y < self.offset_y {
            self.offset_y = self.cursor_y;
        } else if self.cursor_y >= self.offset_y + editor_height {
            self.offset_y = self.cursor_y - editor_height + 1;
        }
    }

    pub fn save(&mut self) -> Result<bool> {
        if let Some(filename) = &self.filename {
            let contents = self.lines.join("\n");
            fs::write(filename, contents)?;
            self.modified = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct Editor {
    buffer: TextBuffer,
    terminal_size: (u16, u16),
    show_line_numbers: bool,
    should_quit: bool,
    status_message: String,
}

impl Editor {
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

    pub fn run(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, Hide)?;

        let result = self.main_loop();

        execute!(stdout(), LeaveAlternateScreen, Show)?;
        terminal::disable_raw_mode()?;

        result
    }

    fn main_loop(&mut self) -> Result<()> {
        loop {
            self.refresh_screen()?;

            if self.should_quit {
                break;
            }

            // 使用事件轮询而非阻塞读取
            // 这样可以更好地控制事件处理的频率
            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key_event) = event::read()? {
                    // 确保每个按键事件只被处理一次
                    self.process_key(key_event)?;
                    // 清除状态消息
                    if !self.status_message.is_empty() {
                        self.status_message.clear();
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

    fn process_key(&mut self, key_event: KeyEvent) -> Result<()> {
        // 记录按键处理，用于调试
        // println!("Processing key: {:?}", key_event);
        
        match key_event {
            KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => {
                if self.buffer.modified && self.status_message.contains("File modified") {
                    // Second Ctrl+X - exit without saving
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
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.move_cursor(Direction::Up, self.terminal_size);
            }
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.move_cursor(Direction::Down, self.terminal_size);
            }
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.move_cursor(Direction::Left, self.terminal_size);
            }
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.buffer.move_cursor(Direction::Right, self.terminal_size);
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
                // 直接调用delete_char，确保不会被重复调用
                self.buffer.delete_char();
            }
            KeyEvent {
                code: KeyCode::Char(ch),
                modifiers,
                ..
            } if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
                // 直接调用insert_char，确保不会被重复调用
                self.buffer.insert_char(ch);
            }
            _ => {}
        }
        Ok(())
    }

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

        // Position cursor
        let line_number_width = if self.show_line_numbers { 4 } else { 0 };
        let screen_x = (self.buffer.cursor_x - self.buffer.offset_x + line_number_width) as u16;
        let screen_y = (self.buffer.cursor_y - self.buffer.offset_y) as u16;
        execute!(stdout(), cursor::MoveTo(screen_x, screen_y))?;
        
        stdout().flush()?;
        Ok(())
    }

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
        let status = format!(" {} - {} lines{}", filename, self.buffer.lines.len(), modified_indicator);
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

    fn draw_help_bar(&self) -> Result<()> {
        let (width, height) = self.terminal_size;
        execute!(stdout(), cursor::MoveTo(0, height - 1))?;
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
        
        let help = "^X Exit  ^O Save  ^W Search  ^K Cut  ^U Paste";
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

fn main() -> Result<()> {
    let args = Args::parse();
    let mut editor = Editor::new(args)?;
    editor.run()
}