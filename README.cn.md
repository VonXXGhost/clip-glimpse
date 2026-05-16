# ClipGlimpse

> [English version](README.md)

通过屏幕上的二维码在隔离云桌面之间传输文本。

**问题**：云桌面阻止回向流量（云端 → 本地），但本地 → 云端正常。传统剪贴板/文件传输是单向的。ClipGlimpse 仅通过视觉通道桥接这一缺口。

**方案**：在云端将文本编码为二维码分块并循环显示，在本地捕获屏幕、解码并重组文本——全离线、全合规。

## 架构

```
┌──────────────────────┐         ┌──────────────────────┐
│   云端 PC             │         │   本地 PC             │
│                      │  屏幕   │                      │
│  生成模式             │◄────────│  读取模式             │
│  • 文本 → QR 分块    │  远程   │  • 屏幕捕获           │
│  • 循环显示          │  桌面   │  • 二维码解码         │
│  • 可配置速度        │         │  • 文本重组           │
└──────────────────────┘         │  • 自动写入剪贴板     │
                                 │  • 系统通知           │
                                 │  • 内存历史记录       │
                                 └──────────────────────┘
```

## 快速开始

### 生成模式（在云端 PC 上运行）

```bash
clip_glimpse generate
```

打开 GUI 窗口：
1. 在输入区粘贴文本
2. 选择预设和显示间隔
3. 文本变化后自动开始循环显示二维码
4. 让本地设备的屏幕捕获对准二维码区域

### 读取模式（在本地 PC 上运行）

```bash
clip_glimpse read
```

1. **首次运行**：拖拽选择二维码显示的屏幕区域
2. 按下 **Ctrl+Shift+V** 或点击 **开始扫描** 开始扫描
3. 收到完整消息后自动：复制到剪贴板 → 弹出系统通知 → 停止扫描
4. 消息可在 **历史记录** 标签页查看——选中预览，点击 **复制到剪贴板**

## 传输协议

每个二维码携带一个结构化的二进制数据块：

```
┌────────┬────────┬──────────┬─────────┬─────────┬──────────────────┐
│  MAGIC │  TYPE  │ VERSION  │   SEQ   │  TOTAL  │    PAYLOAD       │
│  2 字节 │ 1 字节 │  1 字节  │  2 字节  │  2 字节  │   (N 字节)       │
│  "CG"  │ S/D/E  │   0x01   │ u16 BE  │ u16 BE  │   UTF-8 文本     │
└────────┴────────┴──────────┴─────────┴─────────┴──────────────────┘
```

| 类型 | 字节 | 说明 |
|------|------|------|
| SOS | `0x53` | 消息起始，携带整条消息的 CRC32 |
| DATA | `0x44` | 数据片段（UTF-8 文本段） |
| EOS | `0x45` | 消息结束，携带 CRC32 用于校验 |

读取器通过 `(TYPE, SEQ, TOTAL)` 去重和排序，重组后校验 CRC32 再交付。单条消息最多 100 个分块。

## 预设

| 预设 | 版本 | 纠错级别 | 像素/模块 | 每块负载 | 显示尺寸 |
|------|------|----------|-----------|----------|----------|
| Conservative V20-Q | V20 | Q | 3 px | 419 B | ~291×291 |
| **Default V25-M** | **V25** | **M** | **3 px** | **771 B** | **~351×351** |
| Fast V30-M | V30 | M | 2 px | 1035 B | ~274×274 |
| Extreme V35-L | V35 | L | 2 px | 1587 B | ~314×314 |

负载大小已减去 8 字节协议头。

## 吞吐量（估算）

| 文本大小 | Default V25-M | Fast V30-M | Extreme V35-L |
|----------|---------------|------------|---------------|
| 1 KB | ~0.6 秒 | ~0.3 秒 | ~0.3 秒 |
| 10 KB | ~4.2 秒 | ~3.0 秒 | ~2.1 秒 |
| 100 KB | ~40 秒 | ~30 秒 | ~20 秒 |
| 1 MB | ~6 分 44 秒 | ~5 分 2 秒 | ~3 分 17 秒 |

基于 200 ms 扫描间隔。实际性能受屏幕捕获速度和二维码图像质量影响。

## 配置

配置文件为工作目录下的 `config.toml`。完整选项见 [config.example.toml](config.example.toml)。

主要配置项：
- `scan_interval_ms` — 扫描轮询间隔（默认 200 毫秒）
- `hotkey` — 热键字符串，如 `"Ctrl+Shift+V"`（不区分大小写）
- `hotkey_enabled` — 是否启用热键切换（默认 true）
- `log_enabled` — 是否写入 `clip_glimpse.log`（默认 true）
- `generate_preset_index` — 生成模式默认预设（默认 1 = V25-M）
- `generate_interval_ms` — 生成模式循环间隔（默认 500 毫秒）

## 功能特性

- **自动分离**：首次运行自动启动子进程并加上 `--detached` 参数后退出，让命令行立即返回提示符
- **热键轮询**：通过可配置热键（默认 `Ctrl+Shift+V`）切换扫描，采用边沿触发避免重复触发
- **自动写入剪贴板**：完整消息自动通过 `SetClipboardData(CF_UNICODETEXT)` 写入 Windows 剪贴板
- **系统通知**：通过 `Shell_NotifyIconW` 在消息完成时弹出系统气泡通知
- **自动停止**：收到完整消息后自动停止扫描，避免持续占用资源
- **SOS 超时**：如果 30 秒内未收到 EOS，组装器自动重置，等待下一条消息
- **区域重选**：点击扫描面板中的"更换区域"可在不重启的情况下重新选择捕获区域
- **历史记录**：内存中的消息历史（最多 100 条），在历史标签页中查看和复制
- **循环显示**：生成模式下二维码帧以可配置间隔（200/300/500/800/1000 毫秒）循环切换
- **CJK 字体**：自动加载黑体、宋体或微软雅黑以支持中文渲染

## 开发

### 环境要求

- Rust 1.75+（2021 edition）
- Windows 10/11（使用 GDI 屏幕捕获和热键轮询）
- MSVC 工具链（`stable-x86_64-pc-windows-msvc`）

### 编译

```bash
cargo build --release
```

或指定 MSVC 工具链：

```bash
cargo +stable-x86_64-pc-windows-msvc build --release
```

### 测试

```bash
cargo test
```

或：

```bash
cargo +stable-x86_64-pc-windows-msvc test
```

### 模块结构

```
src/
├── main.rs              # 入口，进程分离逻辑
├── cli.rs               # 命令行参数解析（clap）
├── protocol.rs          # 二进制分块编码/解码、CRC32、消息重组
├── qr_gen.rs            # 二维码图像生成（qrcode crate）
├── qr_read.rs           # 从像素数据解码二维码（rxing crate）
├── screen.rs            # Windows GDI 屏幕捕获（BitBlt, GetDIBits）
├── hotkey.rs            # 全局热键解析和轮询（GetAsyncKeyState）
├── clipboard.rs         # Win32 剪贴板写入（SetClipboardData）
├── notify.rs            # Windows 通知（Shell_NotifyIconW）
├── history.rs           # 内存消息历史（最多 100 条）
├── logger.rs            # 文件日志（clip_glimpse.log）
├── icon.rs              # 程序图标生成
├── tray.rs              # 系统托盘（待集成）
├── generate/
│   ├── mod.rs           # 生成模式入口 + 字体加载
│   └── ui.rs            # 生成模式 GUI（eframe/egui）
└── read/
    ├── mod.rs           # 读取模式入口 + 配置 + 热键线程
    ├── ui.rs            # 读取模式 GUI（eframe/egui）
    ├── scanner.rs       # 后台扫描线程：捕获 → 解码 → 组装 → 通知
    └── region.rs        # 全屏区域选择覆盖层
```

### 主要依赖

| 用途 | 库 |
|------|-----|
| GUI | `eframe` / `egui` |
| 二维码生成 | `qrcode` |
| 二维码解码 | `rxing` |
| 屏幕捕获 | `windows`（GDI） |
| 命令行 | `clap` |
| 图像处理 | `image` |
| 序列化 | `serde` + `toml` |
| 时间 | `chrono` |

## 使用

编译完成后，可执行文件位于：

```
target/release/clip_glimpse.exe
```

**生成模式**（在云端 PC 上）：
```bash
clip_glimpse generate
```

**读取模式**（在本地 PC 上）：
```bash
clip_glimpse read
```

## 许可证

MIT
