# Black Hole Trash

**简体中文** | [English](README.en.md)

[![Latest Release](https://img.shields.io/github/v/release/rrrjqy66/BlackHoleTrash?label=release)](https://github.com/rrrjqy66/BlackHoleTrash/releases/latest)
[![Stars](https://img.shields.io/github/stars/rrrjqy66/BlackHoleTrash)](https://github.com/rrrjqy66/BlackHoleTrash/stargazers)
[![License](https://img.shields.io/github/license/rrrjqy66/BlackHoleTrash)](LICENSE)

Black Hole Trash 是一个小巧、可拖动的 Windows 桌面黑洞，也是一个真正的回收站入口。它用实时引力透镜扭曲桌面，把文件、文件夹或多个项目安全送入 Windows 回收站，而不是永久删除。

## 下载 v1.2.0

[下载 BlackHoleTrash-Setup-x64.exe](https://github.com/rrrjqy66/BlackHoleTrash/releases/download/v1.2.0/BlackHoleTrash-Setup-x64.exe) · [查看 Release](https://github.com/rrrjqy66/BlackHoleTrash/releases/tag/v1.2.0)

安装包按当前用户安装，不需要管理员权限。安装包尚未进行代码签名，Windows SmartScreen 可能显示安全提醒；可在同一 Release 页面下载 `.sha256` 文件核对完整性。

## 当前功能

- **真实回收站拖放**：通过 Windows OLE `IDropTarget` 接收文件、文件夹和多项目拖放，使用 `IFileOperation + FOFX_RECYCLEONDELETE` 送入回收站。
- **不会永久删除**：没有 `DeleteFile`、`RemoveDirectory` 或 `std::fs::remove_*` 后备路径。Shell 拒绝或取消时，源文件保持原状。
- **实时引力透镜**：Schwarzschild / Kerr 测地线、事件视界、光子环、相对论吸积盘和黑体辐射实时渲染。
- **8 种外观预设**：Inferno、Gargantua、Quasar、M87* donut、Blazar、Face-on ember、Pure lens、Zen，可从托盘实时切换并平滑过渡。
- **鼠标引力与吸收残影**：光标进入引力场后受到渐进阻力，靠近事件视界时绕转并带着多层方向残影吸收；快速向外甩动可脱离。
- **六档吞噬成长**：吞噬数量达到 `0 / 1 / 3 / 5 / 10 / 20` 个时切换尺寸，最大为初始尺寸的 `4 倍`，每次变化带平滑过渡。
- **中文系统托盘**：外观、大小、速度、帧率、屏幕保护、旋转、位置和显示器等选项均可从托盘调整。
- **固定帧率选项**：托盘只提供 `30 帧` 和 `60 帧`；配置缺失、`0`、无效值或其他数值都会回退到 `60`。
- **实时桌面捕获**：Windows 使用 DX12、`windows-capture` 与 D3D11 → D3D12 共享纹理零拷贝路径，并保留 CPU 后备路径。
- **多显示器漫游**：每台显示器使用独立 Pane，黑洞可跨屏移动，也可从托盘固定到指定显示器。
- **屏幕保护模式**：键盘鼠标空闲指定分钟后出现，首次输入时消失。
- **捕获排除与截图修复**：覆盖窗口不会被自身桌面捕获再次采集，避免无限镜像；全屏 Print Screen 截图可将黑洞合成回剪贴板图像。
- **更新提示**：每天最多检查一次 GitHub Release，并在托盘中提示新版本；可在配置中关闭。

## v1.2.0 新增

- **六档尺寸**：新增吞噬 `20` 个物品的第六档，尺寸达到初始大小的 `4.00 倍`；前五档为 `1.00 / 1.25 / 1.50 / 1.75 / 2.00 倍`。
- **托盘汉化**：大小、速度、帧率、屏幕保护、旋转、位置和退出等菜单统一为中文。
- **帧率收敛**：帧率选项只保留 `30 帧`、`60 帧`，移除无限帧分支，默认值和异常回退值均为 `60`。

## 使用方法

1. 启动 `BlackHoleTrash.exe`。
2. 按住黑洞并拖动，将它放到需要的位置；也可以按住 `Ctrl+Shift` 将黑洞固定到鼠标位置。
3. 从 Windows 文件资源管理器拖入文件、文件夹或多个项目。
4. 在黑洞中心松开鼠标，项目会进入 Windows 回收站。
5. 右键系统托盘图标切换外观、大小、速度、帧率、位置和其他设置。

鼠标吸收动画约持续 160 ms，并在黄圈附近开始收束。吸收不会锁死光标：快速向外甩动会立即脱离引力场；吸收后的系统光标最多隐藏 150 ms，并在下一次物理移动时恢复。拖动黑洞本体时吸附会暂时停用，拖入文件时保持生效。

## 安全规则

程序会在拖入和松手时重复校验路径，并拒绝无法确认能安全回收的项目，包括：

- 网络路径和映射网络盘；
- 移动设备和非固定磁盘；
- 磁盘根目录；
- 已位于 `$Recycle.Bin` 内的项目；
- Windows 系统目录；
- Black Hole Trash 自身及其安装目录；
- 已消失、已变化或无法由 Shell 解析的路径。

如果 Shell 拒绝或取消操作，文件保持原状；程序不会改用永久删除。

## 系统要求

- Windows 10 版本 2004 或更高版本；
- Windows 11；
- x64 桌面环境；
- 支持 DirectX 12 的显卡。

正式版本不需要预先安装 Rust、Node.js 或 Python。

## 配置

配置文件名为 `black-hole-trash.toml`，与可执行文件放在同一目录。配置支持热重载，也可以直接使用托盘菜单调整常用选项。

```toml
# 帧率只接受 30 或 60；缺失、无效值和其他数值回退到 60。
fps = 60

# 尺寸会吸附到六档：0.011 / 0.01375 / 0.0165 / 0.01925 / 0.022 / 0.044
size = 0.011

# 外观：inferno | gargantua | quasar | m87 | blazar | ember | lens | zen
preset = gargantua
```

完整配置模板可以从托盘菜单打开配置文件后查看。

## 从源码构建

需要安装 [Rust](https://rustup.rs) 的 MSVC 工具链。

```powershell
cargo build --release --bin BlackHoleTrash
cargo test --all-targets
cargo fmt --all -- --check
cargo check --all-targets
```

输出文件为 `target\release\BlackHoleTrash.exe`。WGSL 主着色器位于 `src/black_hole_trash.wgsl`，Windows 拖放与回收逻辑位于 `src/platform/windows/`。

## 平台支持

- **Windows**：主要平台，已覆盖桌面捕获、拖放回收、托盘和安装包流程。
- **macOS**：ScreenCaptureKit 和菜单栏结构已保留，但没有真实 Mac 开发环境，属于未测试状态。
- **Linux**：暂不支持；X11 和 Wayland 没有可靠的窗口捕获排除路径。

## 历史版本

| 版本 | 功能增加 |
| --- | --- |
| **v1.2.0** | 六档吞噬成长（最大 4 倍）、中文托盘、仅 30/60 帧率选项、异常帧率回退策略。 |
| **v1.1.0** | 鼠标引力、渐进阻力、螺旋吸收、方向残影、快速甩动脱离和跨显示器轨迹。 |
| **v1.0.0** | 首个可公开下载的 Windows x64 安装版本，包含实时引力透镜、Explorer 拖放回收、多显示器和托盘预设。 |

## Star History

[![Star History Chart](assets/community/star-history.svg)](https://www.star-history.com/#rrrjqy66/BlackHoleTrash&Date)

## 项目来源与许可证

Black Hole Trash 基于 [GreenScreen410/singularity](https://github.com/GreenScreen410/singularity) 开发，并保留 MIT 许可证和原作者署名。黑洞概念亦受到 [ghostty-blackhole](https://github.com/s0xDk/ghostty-blackhole) 启发。

MIT，详见 [LICENSE](LICENSE)。

## 交流与投喂

<table>
  <tr>
    <td align="center"><strong>加入 QQ 群</strong><br><img src="assets/community/qq-group.jpg" width="260" alt="QQ 群二维码"><br>群号：1083121107</td>
    <td align="center"><strong>投喂支持</strong><br><img src="assets/community/donate.jpg" width="260" alt="投喂二维码"><br>感谢你的支持</td>
  </tr>
</table>

欢迎 Star、提交 Issue 或加入 QQ 群交流。
