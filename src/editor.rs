mod input;
mod prompt;
mod status;
mod ui;

use crate::args::Args;
use crate::buffer::TextBuffer;
// use crate::direction::Direction; // 未使用，可去掉
use crate::version::AppInfo;
use crate::Result;

pub struct Editor {
    pub buffer: TextBuffer,
    pub terminal_size: (u16, u16),
    pub show_line_numbers: bool,
    pub should_quit: bool,
    pub status_message: String,
    pub file_save_prompt: Option<String>,
    pub file_save_input: String,
    pub exit_confirm_prompt: bool,
    pub app_info: AppInfo,
    pub show_help_page: bool,
    pub help_page_drawn: bool,
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
            app_info,
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

    fn refresh_screen(&mut self) -> Result<()> {
        ui::refresh_screen(self)
    }

    fn draw_help_page(&self) -> Result<()> {
        use crossterm::{
            cursor, execute, style,
            terminal::{self, ClearType},
        };
        use std::io::stdout;
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

    fn main_loop(&mut self) -> Result<()> {
        use crossterm::event::{self, KeyCode};
        loop {
            // 如果正在显示帮助页面
            if self.show_help_page {
                self.draw_help_page()?;
                // 按任意键关闭帮助页面
                if event::poll(std::time::Duration::from_millis(50))? {
                    if let event::Event::Key(key_event) = event::read()? {
                        match key_event.code {
                            KeyCode::Esc
                            | KeyCode::Char(_)
                            | KeyCode::Enter
                            | KeyCode::Backspace => {
                                self.show_help_page = false;
                                self.status_message.clear();
                            }
                            _ => {}
                        }
                    }
                }
                continue; // 跳过后续刷新和输入处理
            }

            self.refresh_screen()?;
            if self.should_quit {
                break;
            }
            if event::poll(std::time::Duration::from_millis(50))? {
                if let event::Event::Key(key_event) = event::read()? {
                    if key_event.kind == event::KeyEventKind::Press {
                        input::process_key(self, key_event)?;
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
}
