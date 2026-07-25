# 管理员权限启动拦截设计

## 行为

Windows 版在 `main` 的最前面读取当前进程令牌的 `TokenElevation`。普通权限继续启动；管理员权限弹出警告并立即退出。权限检测失败时也提示并退出，避免进入拖放能力不确定的状态。

警告明确说明：Windows UIPI 会阻止普通权限的文件资源管理器向管理员权限黑洞拖放文件；Black Hole Trash 不需要管理员权限；请直接双击程序或从开始菜单普通启动。

检查必须早于单实例接管、事件循环、GPU、托盘和 OLE 初始化，确保误启动的管理员实例不会关闭已运行的普通权限实例。

## 代码

- 新增 `src/platform/windows/process_elevation.rs`：封装 `OpenProcessToken`、`GetTokenInformation(TokenElevation)`、错误消息和警告弹窗。
- `src/platform/windows/mod.rs` 导出启动权限守卫。
- `src/main.rs` 在 `env_logger::init()` 后立即调用守卫，返回 `false` 时退出。
- `README.md` 和 `README.en.md` 增加故障排查说明。

不实现自动降权重启，不允许管理员实例带警告继续运行，不修改安装器的 `PrivilegesRequired=lowest`。

## 验证

- 单元测试覆盖 `TokenIsElevated = 0`、非零值、中文提示内容和检测失败提示。
- 普通权限 Release 可正常启动。
- 管理员权限 Release 只显示警告，不创建 Black Hole Trash 进程窗口、托盘或拖放目标。
- 运行格式检查、完整测试、Clippy、所有目标检查和 Release 构建。
