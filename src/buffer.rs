use std::fs;
use std::path::PathBuf;

use crate::direction::Direction;
use crate::Result;

/// 文本缓冲区，存储编辑器的内容和光标状态
#[derive(Clone)]
pub struct TextBuffer {
    pub lines: Vec<String>,
    /// 光标所在字符索引（不是字节索引，支持中文）
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub cursor_x2: Option<usize>,
    pub cursor_y2: Option<usize>,
    pub offset_x: usize,
    pub offset_y: usize,
    pub modified: bool,
    pub filename: Option<PathBuf>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            cursor_x2: None,
            cursor_y2: None,
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
            cursor_x2: None,
            cursor_y2: None,
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

    /// 在当前光标位置插入字符（按字符索引插入，支持中文）
    pub fn insert_char(&mut self, ch: char) {
        let line = self.current_line_mut();
        let mut byte_pos = 0;
        let mut char_count = 0;
        for (i, (pos, _)) in line.char_indices().enumerate() {
            if char_count == self.cursor_x {
                byte_pos = pos;
                break;
            }
            char_count += 1;
        }
        if self.cursor_x >= line.chars().count() {
            byte_pos = line.len();
        }
        line.insert(byte_pos, ch);
        self.cursor_x += 1;
        self.modified = true;
    }

    /// 插入新行，光标移到下一行行首
    pub fn insert_newline(&mut self) {
        let line = self.current_line().clone();
        let mut byte_pos = 0;
        let mut char_count = 0;
        for (i, (pos, _)) in line.char_indices().enumerate() {
            if char_count == self.cursor_x {
                byte_pos = pos;
                break;
            }
            char_count += 1;
        }
        if self.cursor_x >= line.chars().count() {
            byte_pos = line.len();
        }
        let (left, right) = line.split_at(byte_pos);
        self.lines[self.cursor_y] = left.to_string();
        self.lines.insert(self.cursor_y + 1, right.to_string());
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.modified = true;
    }

    /// 删除光标前字符（支持中文，按字符索引删除）
    pub fn delete_char(&mut self) {
        if self.cursor_x > 0 {
            let line = self.current_line_mut();
            let mut byte_pos = 0;
            let mut char_indices: Vec<usize> = line.char_indices().map(|(i, _)| i).collect();
            if self.cursor_x < char_indices.len() {
                byte_pos = char_indices[self.cursor_x];
            } else {
                byte_pos = line.len();
            }
            let prev_pos = if self.cursor_x > 0 {
                char_indices[self.cursor_x - 1]
            } else {
                0
            };
            line.drain(prev_pos..byte_pos);
            self.cursor_x -= 1;
            self.modified = true;
        } else if self.cursor_y > 0 {
            // 与上一行合并
            let current_line = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].chars().count();
            self.lines[self.cursor_y].push_str(&current_line);
            self.modified = true;
        }
    }

    /// 光标移动，支持左右行首/行尾跳转
    pub fn move_cursor(&mut self, direction: Direction, terminal_size: (u16, u16), is_secondary: bool) {
        let lines_len = self.lines.len();
        if is_secondary {
            let (x, y) = match (self.cursor_x2, self.cursor_y2) {
                (Some(x), Some(y)) => (x, y),
                _ => {
                    self.cursor_x2 = Some(self.cursor_x);
                    self.cursor_y2 = Some(self.cursor_y);
                    (self.cursor_x, self.cursor_y)
                }
            };
            let line_len = self.lines[y].chars().count();
            match direction {
                Direction::Up => {
                    if y > 0 {
                        self.cursor_y2 = Some(y - 1);
                        let up_len = self.lines[y - 1].chars().count();
                        self.cursor_x2 = Some(x.min(up_len));
                    }
                }
                Direction::Down => {
                    if y < lines_len - 1 {
                        self.cursor_y2 = Some(y + 1);
                        let down_len = self.lines[y + 1].chars().count();
                        self.cursor_x2 = Some(x.min(down_len));
                    }
                }
                Direction::Left => {
                    if x > 0 {
                        self.cursor_x2 = Some(x - 1);
                    } else if y > 0 {
                        self.cursor_y2 = Some(y - 1);
                        let prev_len = self.lines[y - 1].chars().count();
                        self.cursor_x2 = Some(prev_len);
                    }
                }
                Direction::Right => {
                    if x < line_len {
                        self.cursor_x2 = Some(x + 1);
                    } else if y < lines_len - 1 {
                        self.cursor_y2 = Some(y + 1);
                        self.cursor_x2 = Some(0);
                    }
                }
            }
        } else {
            let line_len = self.current_line().chars().count();
            match direction {
                Direction::Up => {
                    if self.cursor_y > 0 {
                        self.cursor_y -= 1;
                        let up_len = self.current_line().chars().count();
                        self.cursor_x = self.cursor_x.min(up_len);
                    }
                }
                Direction::Down => {
                    if self.cursor_y < lines_len - 1 {
                        self.cursor_y += 1;
                        let down_len = self.current_line().chars().count();
                        self.cursor_x = self.cursor_x.min(down_len);
                    }
                }
                Direction::Left => {
                    if self.cursor_x > 0 {
                        self.cursor_x -= 1;
                    } else if self.cursor_y > 0 {
                        self.cursor_y -= 1;
                        self.cursor_x = self.current_line().chars().count();
                    }
                }
                Direction::Right => {
                    if self.cursor_x < line_len {
                        self.cursor_x += 1;
                    } else if self.cursor_y < lines_len - 1 {
                        self.cursor_y += 1;
                        self.cursor_x = 0;
                    }
                }
            }
        }
        // 滚动逻辑略
        let (_, height) = terminal_size;
        let editor_height = height as usize - 2;
        if self.cursor_y < self.offset_y {
            self.offset_y = self.cursor_y;
        } else if self.cursor_y >= self.offset_y + editor_height {
            self.offset_y = self.cursor_y - editor_height + 1;
        }
        if let Some(y2) = self.cursor_y2 {
            if y2 < self.offset_y {
                self.offset_y = y2;
            } else if y2 >= self.offset_y + editor_height {
                self.offset_y = y2 - editor_height + 1;
            }
        }
    }

    /// 保存缓冲区内容到文件
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
    
    /// 切换第二个光标的显示/隐藏
    pub fn toggle_secondary_cursor(&mut self) {
        if self.cursor_x2.is_some() && self.cursor_y2.is_some() {
            // 隐藏第二个光标
            self.cursor_x2 = None;
            self.cursor_y2 = None;
        } else {
            // 显示第二个光标，初始位置与主光标相同
            self.cursor_x2 = Some(self.cursor_x);
            self.cursor_y2 = Some(self.cursor_y);
        }
    }
    
    /// 同时在两个光标位置插入字符
    pub fn insert_char_at_both_cursors(&mut self, ch: char) {
        // 先在主光标位置插入
        self.insert_char(ch);
        
        // 再在第二个光标位置插入
        if let (Some(x2), Some(y2)) = (self.cursor_x2, self.cursor_y2) {
            // 保存当前主光标位置
            let main_x = self.cursor_x;
            let main_y = self.cursor_y;
            
            // 临时切换到第二个光标位置
            self.cursor_x = x2;
            self.cursor_y = y2;
            
            // 插入字符
            self.insert_char(ch);
            
            // 恢复主光标位置
            self.cursor_x = main_x;
            self.cursor_y = main_y;
        }
    }
}