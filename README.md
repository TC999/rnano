# RSNano

> [!caution]
>
> 请注意，本仓库不是加密货币。
> 本项目是一个简化版的编辑器，功能上可能与完整的 nano 编辑器有所不同。

RSNano 是一个用 Rust 语言实现的[ GNU nano 编辑器](https://www.nano-editor.org)的简化版。它致力于以更现代的编程语言重现经典的命令行文本编辑体验，拥有轻量、快速、安全等特点，适合在各种终端环境下使用。支持 Linux 以及 Windows 系统。

## 项目特点

- **纯 Rust 实现**：高安全性与性能，代码更易维护。
- **GNU nano 编辑器风格**：操作习惯与 nano 类似，适合习惯 nano 的用户。
- **基本文本编辑功能**：支持打开、编辑、保存文本文件。
- **行号显示（可选）**：更方便代码或文本查看与定位。
- **简洁快捷键操作**：支持常用 nano 编辑器快捷键。

## 快速开始

### 构建与运行

1. 安装 Rust 环境（推荐使用 [rustup](https://rustup.rs/)）。
2. 克隆项目并编译运行：

```bash
git clone https://github.com/TC999/rsnano.git
cd rsnano
cargo build --release
cargo run --release [文件名] [--line-numbers]
```

### 命令行参数

- `[文件名]` 可选，指定要编辑的文件。
<!-- - `--line-numbers` 可选，显示行号。 -->

## 主要快捷键

- `Ctrl+X`：退出编辑器（若文件已修改需按两次）
- `Ctrl+O`：保存文件
- `方向键`：移动光标
- `Enter`：插入新行
- `Backspace`：删除字符

## 更新与修复

- 修复了 `Ctrl+X` 无法正确退出的问题
- 修复了 Windows 系统下字符重复输入的问题
- 其他小问题持续改进中
3. **帮助页面频闪问题**
   - 修复了按下 Ctrl+G 打开帮助页时的屏幕频闪问题
   - 通过优化屏幕刷新逻辑，避免不必要的屏幕重绘



## 贡献方式

欢迎任何形式的贡献，包括代码、文档、测试及建议。请在 [议题区](https://github.com/TC999/rsnano/issues) 提交你的问题或想法。

## 许可协议

本项目采用 GPL3 协议，详情请见 [`LICENSE`](./LICENSE)。

---

如需更多帮助或有疑问，请通过议题联系作者。