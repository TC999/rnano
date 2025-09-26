use clap::Parser;
use std::path::PathBuf;

use crate::Result;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// 要编辑的文件
    pub file: Option<PathBuf>,
    
    /// 显示行号
    #[arg(short, long)]
    pub line_numbers: bool,
}

impl Args {
    /// 从命令行参数解析Args实例
    pub fn from_cli() -> Result<Self> {
        Self::try_parse_from(std::env::args_os()).map_err(|e| {
            Box::new(e) as Box<dyn std::error::Error>
        })
    }
}