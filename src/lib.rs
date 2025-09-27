// RSNano编辑器库入口点

// 导出各个模块
pub mod buffer;
pub mod editor;
pub mod direction;
pub mod version;
pub mod args;

// 定义Result类型别名
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;