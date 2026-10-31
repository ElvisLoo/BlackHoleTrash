#!/usr/bin/env bash
# BlackHoleTrash 构建脚本（Windows / Git Bash）
#
# 为什么需要它：这台机器上 cargo 默认用 MSVC 目标，但缺 dbghelp.lib；
# GNU 目标又需要 MinGW 的 dlltool。手工配 PATH 少一段就报"program not found"，
# 而且要保证 MSVC 的 link.exe 排在 Git 自带的 /usr/bin/link 前面。
#
# 用法：
#   ./build.sh check   # 只做类型检查（快）
#   ./build.sh test    # 跑测试
#   ./build.sh build   # 出 exe

set -euo pipefail

CARGO_BIN="/c/Users/Admin/.cargo/bin"
MINGW_BIN="$(ls -d /c/Users/Admin/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.*/mingw64/bin 2>/dev/null | head -1)"
MSVC_BIN="$(ls -d "/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/"*/bin/Hostx64/x64 2>/dev/null | head -1)"

if [ -z "$MINGW_BIN" ]; then
  echo "缺少 MinGW（需要 dlltool.exe）。安装：winget install BrechtSanders.WinLibs.POSIX.UCRT" >&2
  exit 1
fi

export PATH="$CARGO_BIN:$MINGW_BIN:${MSVC_BIN:+$MSVC_BIN:}$PATH"

# 确认 dlltool 能找到，否则构建会在链接阶段才失败，浪费几分钟
command -v dlltool.exe >/dev/null 2>&1 || { echo "dlltool 不在 PATH 上" >&2; exit 1; }

TARGET="+stable-x86_64-pc-windows-gnu"
cd "$(dirname "$0")"

case "${1:-build}" in
  check)  cargo $TARGET check ;;
  test)   cargo $TARGET test ;;
  build)  cargo $TARGET build ;;
  release) cargo $TARGET build --release ;;
  *) echo "用法: $0 [check|test|build|release]" >&2; exit 2 ;;
esac
