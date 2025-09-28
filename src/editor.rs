mod input;
mod prompt;
mod status;
mod ui;

use crossterm::terminal::ClearType;
use crossterm::{cursor, execute, style, terminal};
use std::io::stdout;

use crate::args::Args;
use crate::buffer::TextBuffer;
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
            // Handle help page display separately from main loop
            if self.show_help_page {
                if !self.help_page_drawn {
                    self.draw_help_page()?;
                    self.help_page_drawn = true;
                }
                if crossterm::event::poll(std::time::Duration::from_millis(50))? {
                    if let crossterm::event::Event::Key(_) = crossterm::event::read()? {
                        self.show_help_page = false;
                        self.help_page_drawn = false;
                    }
                }
                continue;
            }

            ui::refresh_screen(self)?;
            if self.should_quit {
                break;
            }
            if crossterm::event::poll(std::time::Duration::from_millis(50))? {
                if let crossterm::event::Event::Key(key_event) = crossterm::event::read()? {
                    if key_event.kind == crossterm::event::KeyEventKind::Press {
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
        // Ensure the output is immediately visible
        use std::io::Write;
        std::io::stdout().flush()?;
        Ok(())
    }
}
