use std::fs;
use std::path::PathBuf;

use crate::direction::Direction;
use crate::Result;

/// 文本缓冲区，存储编辑器的内容和光标状态
#[derive(Clone)]
pub struct TextBuffer {
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    // 第二个光标的坐标
    pub cursor_x2: Option<usize>,
    pub cursor_y2: Option<usize>,
    pub offset_x: usize,
    pub offset_y: usize,
    pub modified: bool,
    pub filename: Option<PathBuf>,
}

impl TextBuffer {
    /// 创建一个新的空文本缓冲区
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

    /// 从文件创建文本缓冲区
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

    /// 获取当前行的不可变引用
    pub fn current_line(&self) -> &String {
        &self.lines[self.cursor_y]
    }

    /// 获取当前行的可变引用
    pub fn current_line_mut(&mut self) -> &mut String {
        &mut self.lines[self.cursor_y]
    }

    /// 在当前光标位置插入字符
    pub fn insert_char(&mut self, ch: char) {
        // 先保存cursor_x的值，避免借用冲突
        let cursor_x = self.cursor_x;
        let line = self.current_line_mut();
        
        // 安全检查：确保索引是有效的UTF-8字符边界
        let safe_position = if cursor_x > line.len() {
            line.len()
        } else if line.is_char_boundary(cursor_x) {
            cursor_x
        } else {
            // 找到最近的有效的UTF-8字符边界
            let mut pos = cursor_x;
            while pos > 0 && !line.is_char_boundary(pos) {
                pos -= 1;
            }
            pos
        };
        
        // 检查当前位置是否已有相同字符（防止重复）
        if safe_position < line.len() {
            let char_at_pos = line
                .char_indices()
                .skip_while(|&(i, _)| i < safe_position)
                .next()
                .map(|(_, c)| c)
                .unwrap_or('\0');
            
            if char_at_pos == ch {
                return; // 如果字符相同且位置相同，不执行插入
            }
        }
        
        line.insert(safe_position, ch);
        self.cursor_x = safe_position + ch.len_utf8();
        
        // 如果有第二个光标，需要更新其位置
        if let Some(x2) = &mut self.cursor_x2 {
            if self.cursor_y2 == Some(self.cursor_y) && *x2 >= cursor_x {
                *x2 += ch.len_utf8();
            }
        }
        
        self.modified = true;
    }

    /// 在当前光标位置插入新行
    pub fn insert_newline(&mut self) {
        let current_line = self.current_line().clone();
        let safe_position = if self.cursor_x > current_line.len() {
            current_line.len()
        } else if current_line.is_char_boundary(self.cursor_x) {
            self.cursor_x
        } else {
            // 找到最近的有效的UTF-8字符边界
            let mut pos = self.cursor_x;
            while pos > 0 && !current_line.is_char_boundary(pos) {
                pos -= 1;
            }
            pos
        };
        
        let (left, right) = current_line.split_at(safe_position);
        
        self.lines[self.cursor_y] = left.to_string();
        self.lines.insert(self.cursor_y + 1, right.to_string());
        
        self.cursor_y += 1;
        self.cursor_x = 0;
        
        // 更新第二个光标位置
        if let Some(y2) = &mut self.cursor_y2 {
            if *y2 > self.cursor_y - 1 {
                *y2 += 1;
            }
        }
        
        self.modified = true;
    }

    /// 删除光标前的字符
    pub fn delete_char(&mut self) {
        if self.cursor_x > 0 {
            // 先保存cursor_x的值，避免借用冲突
            let cursor_x = self.cursor_x;
            let line = self.current_line_mut();
            
            // 安全检查：确保索引是有效的UTF-8字符边界
            let safe_position = if cursor_x > line.len() {
                line.len()
            } else if line.is_char_boundary(cursor_x) {
                cursor_x
            } else {
                // 找到最近的有效的UTF-8字符边界
                let mut pos = cursor_x;
                while pos > 0 && !line.is_char_boundary(pos) {
                    pos -= 1;
                }
                pos
            };
            
            // 确保不会重复删除
            if safe_position > 0 {
                // 找到要删除的字符的起始位置
                let char_start = line
                    .char_indices()
                    .rev()
                    .skip_while(|&(i, _)| i >= safe_position)
                    .next()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                
                // 删除整个字符
                line.drain(char_start..safe_position);
                self.cursor_x = char_start;
                
                // 更新第二个光标位置
                if let Some(x2) = &mut self.cursor_x2 {
                    if self.cursor_y2 == Some(self.cursor_y) && *x2 >= cursor_x {
                        *x2 = x2.saturating_sub(safe_position - char_start);
                    }
                }
                
                self.modified = true;
            }
        } else if self.cursor_y > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.current_line().len();
            
            // 先获取当前行长度，避免借用冲突
            let current_line_len = self.current_line().len();
            
            // 更新第二个光标位置
            if let (Some(x2), Some(y2)) = (&mut self.cursor_x2, &mut self.cursor_y2) {
                if *y2 > self.cursor_y + 1 {
                    *y2 -= 1;
                } else if *y2 == self.cursor_y + 1 {
                    *x2 = current_line_len + *x2;
                    *y2 -= 1;
                }
            }
            
            self.current_line_mut().push_str(&current_line);
            self.modified = true;
        }
    }

    /// 移动光标
    pub fn move_cursor(&mut self, direction: Direction, terminal_size: (u16, u16), is_secondary: bool) {
        if is_secondary {
            // 处理第二个光标的移动
            let (x, y) = match (self.cursor_x2, self.cursor_y2) {
                (Some(x), Some(y)) => (x, y),
                // 如果第二个光标不存在，则创建它并放在主光标旁边
                _ => {
                    self.cursor_x2 = Some(self.cursor_x);
                    self.cursor_y2 = Some(self.cursor_y);
                    (self.cursor_x, self.cursor_y)
                }
            };
            
            let new_x = x;
            let new_y = y;
            
            match direction {
                Direction::Up => {
                    if new_y > 0 {
                        self.cursor_y2 = Some(new_y - 1);
                        // 确保光标不会超出行长度
                        self.cursor_x2 = Some(new_x.min(self.lines[new_y - 1].len()));
                    }
                }
                Direction::Down => {
                    if new_y < self.lines.len() - 1 {
                        self.cursor_y2 = Some(new_y + 1);
                        // 确保光标不会超出行长度
                        self.cursor_x2 = Some(new_x.min(self.lines[new_y + 1].len()));
                    }
                }
                Direction::Left => {
                    if new_x > 0 {
                        self.cursor_x2 = Some(new_x - 1);
                    } else if new_y > 0 {
                        self.cursor_y2 = Some(new_y - 1);
                        self.cursor_x2 = Some(self.lines[new_y - 1].len());
                    }
                }
                Direction::Right => {
                    if new_x < self.lines[new_y].len() {
                        self.cursor_x2 = Some(new_x + 1);
                    } else if new_y < self.lines.len() - 1 {
                        self.cursor_y2 = Some(new_y + 1);
                        self.cursor_x2 = Some(0);
                    }
                }
            }
        } else {
            // 处理主光标的移动
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
        }
        
        // Adjust scroll offset if cursor goes off screen
        let (_, height) = terminal_size;
        let editor_height = height as usize - 2; // Reserve space for status bar and help
        
        if self.cursor_y < self.offset_y {
            self.offset_y = self.cursor_y;
        } else if self.cursor_y >= self.offset_y + editor_height {
            self.offset_y = self.cursor_y - editor_height + 1;
        }
        
        // 对第二个光标也进行同样的滚动调整
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