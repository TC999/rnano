mod input;
mod prompt;
mod status;
mod ui;

use crate::args::Args;
use crate::buffer::TextBuffer;
use crate::direction::Direction;
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
                        input::process_key(self, key_event)?;
                    }
                }
            }
            let new_size = crossterm::terminal::size()?;
            if new_size != self.terminal_size {
                self.terminal_size = new_size;
            }
        }
        Ok(())
    }

    fn refresh_screen(&mut self) -> Result<()> {
        ui::refresh_screen(self)
    }
}
