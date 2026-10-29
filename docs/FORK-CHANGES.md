# 本 fork 相对上游的改动清单

> 上游：https://github.com/rrrjqy66/BlackHoleTrash
> 基线：`e2d864f`（v1.4.0，2026-07-29）
> 维护：ElvisLoo

## 为什么要维护这份清单

这个 fork 不是只改了链接，而是有 **15 个独有提交**。上游一旦恢复更新，
`git merge upstream/main` 很可能在 `src/main.rs` 上产生冲突，
而冲突一旦处理错了，功能会在没人注意的情况下消失。

所以每次合并上游后，按下面的清单逐条确认。

## 合并上游的标准流程

```bash
cd BlackHoleTrash
git fetch upstream main            # 用 SSH（https 在当前网络下会 Empty reply）
git log --oneline HEAD..upstream/main   # 先看上游到底加了什么
git merge upstream/main
cargo build                        # 必须编译
cargo test                         # 必须过
```

然后对照下面的清单自测。

> 注意：上游 https 地址在本机常被重置，`git remote` 里 upstream 已改成 SSH。
> 如果 SSH 也失败，先用 `curl -s https://api.github.com/repos/rrrjqy66/BlackHoleTrash/commits?per_page=1`
> 确认上游是否真的有新提交，别对着缓存里的旧数据做判断。

## 我们的功能（合并后逐条验）

| # | 功能 | 验证方式 | 易冲突文件 |
|---|---|---|---|
| 1 | 托盘菜单全中文 | 右键托盘，所有项应为中文 | `src/main.rs` 菜单段 |
| 2 | 开机自启（写注册表 Run） | 勾选后 `reg query HKCU\...\Run` 能看到 | `src/main.rs` |
| 3 | 退出时保存配置 | 改尺寸/位置后退出重启，设置应保留 | `src/main.rs` `save_config` |
| 4 | 预设菜单分组 + M87 照片预设 | 右键 → 预设，应有分类与 M87 | `src/main.rs` `PRESETS` |
| 5 | 尺寸档位放大 1.5 倍 | 切到最大档，视感应比上游更大 | `src/main.rs` `SIZE_TIERS` |
| 6 | 布朗运动闪烁模式 | 托盘开启后黑洞会缓慢漂移并淡入淡出 | `src/main.rs` blink 逻辑 |
| 7 | Win11 PrintScreen 修复（CF_DIBV5） | Win+Prtsc 后剪贴板图片应被修正 | `src/screenshot_fix.rs` |
| 8 | 拖放窗口常驻（128px 热区） | 黑洞可见时拖文件到圆圈附近即可吞噬 | `drop_window.rs` |
| 9 | UIPI 绕过（提权后可拖放） | 管理员启动后拖文件仍可吞噬 | `drop_window.rs` |
| 10 | 移除管理员启动拦截 | 管理员身份启动不再被拦 | `process_elevation.rs` |
| 11 | 配置开关可持久化 | `check_updates=0` 后退出，值应仍为 0 | `src/main.rs` `save_config` |

## 我们修过的上游缺陷

| 问题 | 位置 | 说明 |
|---|---|---|
| 配置开关被写死成 1 | `save_config` | 用户设 0 后每次退出又变回 1，开关形同虚设 |
| 写盘失败静默 | `save_config` | 只读目录下保存一直失败但无提示 |
| 选「不限」帧率卡死 | `frame_interval` | `1.0/0.0 = inf` → `from_secs_f64(inf)` panic |
| 托盘勾选错位 | `fps_option_index` | 60 与 0 都映射到索引 1 |

**如果上游日后自己修了同样的问题**，合并时优先用上游实现，删掉我们的版本，
并从本表移除对应行 —— 避免同一处逻辑维护两份。

## 已知但未修的问题

来自一次完整代码审查（未编译验证，标注了把握程度）：

| 问题 | 位置 | 影响 |
|---|---|---|
| 启动期 `unwrap` 无提示 | `EventLoop::new().unwrap()`、`request_adapter` 等 | 驱动异常/远程桌面下双击毫无反应 |
| 提权后拖放必然失效 | UIPI 限制 | 管理员用户拖文件没反应，且无提示 |
| 多显示器只按第一块屏协商格式 | `surface_capabilities` | 副屏可能黑屏（待验证） |
| 捕获线程不 join | `capture_windows.rs` | 切换显示器后线程短暂堆积 |
| std `Mutex` 与 `parking_lot` 混用 | 多处 | 一处 panic 会导致整程序退出 |
| emoji 文件名无法拖入 | `from_utf16_lossy` | 路径含 emoji 时被替换成 U+FFFD |

改这些之前先确认能编译 —— 本机目前没有 Rust 工具链。
