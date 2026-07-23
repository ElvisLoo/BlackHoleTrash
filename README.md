# Black Hole Trash

**简体中文** | [English](README.en.md)

Black Hole Trash 是一个小巧、可拖动的 Windows 桌面黑洞，同时也是一个真正的回收站入口。它使用实时引力透镜着色器扭曲桌面；把文件、文件夹或多个项目拖进事件视界后，Windows Shell 会将它们送入回收站。

## 主要功能

- **真实回收站**：通过 Windows OLE `IDropTarget` 接收资源管理器拖放，并使用 `IFileOperation + FOFX_RECYCLEONDELETE` 送入回收站。
- **绝不永久删除**：项目中没有 `DeleteFile`、`RemoveDirectory` 或 `std::fs::remove_*` 等永久删除后备路径。
- **小巧且可拖动**：默认大小只比桌面图标略大；按住黑洞并拖动即可改变位置，松开后固定。
- **鼠标引力与吸收残影**：Windows 光标进入引力场后会受到最高 10% 的渐进阻力，靠近事件视界时沿轨道绕转，并带着多层残影螺旋收进黑洞。
- **真实引力透镜**：保留 Schwarzschild / Kerr 测地线、事件视界、光子环和相对论吸积盘。
- **实时桌面捕获**：Windows 使用 DX12、`windows-capture` 和 D3D11 → D3D12 共享纹理零拷贝路径，并保留 CPU 后备路径。
- **多显示器支持**：每台显示器使用独立 Pane，黑洞可跨屏显示。
- **托盘与预设**：提供外观、尺寸、位置、帧率、旋转和屏保模式等托盘选项。
- **捕获排除**：覆盖窗口不会被自身的桌面捕获再次采集，避免无限镜像反馈。

## 系统要求

- Windows 10 2004 或更高版本
- Windows 11
- x64 桌面环境
- 支持 DirectX 12 的显卡

运行正式版本不需要预先安装 Rust、Node.js 或 Python。

## 下载

[下载 BlackHoleTrash-Setup-x64.exe](https://github.com/rrrjqy66/BlackHoleTrash/releases/download/v1.1.0/BlackHoleTrash-Setup-x64.exe)

安装包按当前用户安装，不需要管理员权限。由于尚未进行代码签名，Windows SmartScreen 可能显示安全提醒；可在同一 Release 页面下载 `.sha256` 文件核对完整性。

## 使用方法

1. 启动 `BlackHoleTrash.exe`。
2. 按住黑洞并拖动，将它放到需要的位置。
3. 从 Windows 文件资源管理器拖入文件、文件夹或多个项目。
4. 在黑洞中心松开鼠标，项目会进入 Windows 回收站。
5. 通过系统托盘切换预设、大小、位置或退出程序。

也可以按住 `Ctrl+Shift`，让黑洞跟随鼠标并固定到新位置。

鼠标吸收动画约持续 160 ms，并会在黄圈附近开始收束。吸附不会锁死光标：快速向外甩动会立即脱离引力场；吸收后的系统光标最多隐藏 150 ms，并会在下一次物理移动时立即恢复。拖动黑洞本体时吸附会暂时停用，拖入文件时则保持生效。

## 安全规则

Black Hole Trash 会在拖入和松手时重复校验路径，并拒绝无法确认能安全回收的项目，包括：

- 网络路径和映射网络盘；
- 移动设备和非固定磁盘；
- 磁盘根目录；
- 已位于 `$Recycle.Bin` 内的项目；
- Windows 系统目录；
- Black Hole Trash 自身及其安装目录；
- 已消失、已变化或无法由 Shell 解析的路径。

如果 Shell 拒绝或取消操作，程序会显示错误，文件保持原状；不会改用永久删除。

## 从源码构建

需要安装 [Rust](https://rustup.rs) 的 MSVC 工具链。

```powershell
cargo build --release --bin BlackHoleTrash
```

输出文件：

```text
target\release\BlackHoleTrash.exe
```

常用检查：

```powershell
cargo fmt --all
cargo check --all-targets
cargo run --example validate_shader
```

WGSL 主着色器位于 `src/black_hole_trash.wgsl`，Windows 拖放与回收逻辑位于 `src/platform/windows/`。

## 配置

程序配置文件名为 `black-hole-trash.toml`，与可执行文件放在同一目录。配置支持热重载；也可以直接使用托盘菜单调整常用选项。

## 项目来源

Black Hole Trash 基于 [GreenScreen410/singularity](https://github.com/GreenScreen410/singularity) 开发，并保留其 MIT 许可证和原作者署名。黑洞概念亦受到 [ghostty-blackhole](https://github.com/s0xDk/ghostty-blackhole) 启发。

## 许可证

MIT，详见 [LICENSE](LICENSE)。
