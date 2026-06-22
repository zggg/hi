#!/usr/bin/env bash
# macOS + Linux 编译到 npm/dist/（npm publish 或手动分发都用这一处）
#
# 一次性准备:
#   1. 安装并打开 Docker Desktop
#   2. cargo install cross --git https://github.com/cross-rs/cross
#
# 用法:
#   ./scripts/build-dist.sh                      # 全部平台
#   ./scripts/build-dist.sh darwin-arm64         # 单个平台
#   ./scripts/build-dist.sh darwin-arm64 linux-x64
#
# 平台名（与 npm/bin/hi.js 一致）:
#   darwin-arm64 | darwin-x64 | linux-x64 | linux-arm64
#
# 中间产物在 .build/target/（已 gitignore），不在仓库根目录生成 target/、dist/
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/npm/dist"
export CARGO_TARGET_DIR="$ROOT/.build/target"

# macOS + Linux only (no Windows npm dist)
# 格式: tool:rust_target:dist_filename
ALL_TARGETS=(
  "cargo:aarch64-apple-darwin:hi-darwin-arm64"
  "cargo:x86_64-apple-darwin:hi-darwin-x64"
  "cross:x86_64-unknown-linux-gnu:hi-linux-x64"
  "cross:aarch64-unknown-linux-gnu:hi-linux-arm64"
)

VALID_PLATFORMS=(darwin-arm64 darwin-x64 linux-x64 linux-arm64)

usage() {
  cat <<EOF
用法: $0 [平台 ...]

不传参数时编译全部平台。可指定一个或多个平台:
  darwin-arm64   Apple Silicon macOS
  darwin-x64     Intel macOS
  linux-x64      Linux x86_64（需 cross + Docker）
  linux-arm64    Linux ARM64（需 cross + Docker）

示例:
  $0
  $0 darwin-arm64
  $0 darwin-arm64 linux-x64
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

select_targets() {
  local -a selected=()
  local want entry platform out_name
  for want in "$@"; do
    local ok=false
    for platform in "${VALID_PLATFORMS[@]}"; do
      [[ "$want" == "$platform" ]] && ok=true && break
    done
    if ! $ok; then
      echo "未知平台: $want" >&2
      echo "可选: ${VALID_PLATFORMS[*]}" >&2
      exit 1
    fi
    for entry in "${ALL_TARGETS[@]}"; do
      IFS=: read -r _tool _target out_name <<< "$entry"
      if [[ "$out_name" == "hi-$want" ]]; then
        selected+=("$entry")
        break
      fi
    done
  done
  TARGETS=("${selected[@]}")
}

if [[ $# -eq 0 ]]; then
  TARGETS=("${ALL_TARGETS[@]}")
else
  select_targets "$@"
fi

if [[ ${#TARGETS[@]} -eq 0 ]]; then
  echo "没有可编译的平台" >&2
  exit 1
fi

cd "$ROOT"
mkdir -p "$DIST"

need_cross=false
for entry in "${TARGETS[@]}"; do
  [[ "${entry%%:*}" == cross ]] && need_cross=true && break
done

if $need_cross; then
  if ! command -v cross >/dev/null 2>&1; then
    echo "未找到 cross，请先执行:" >&2
    echo "  cargo install cross --git https://github.com/cross-rs/cross" >&2
    exit 1
  fi
  if ! docker info >/dev/null 2>&1; then
    echo "Docker 未运行，请先打开 Docker Desktop" >&2
    exit 1
  fi
fi

echo "==> 安装 Rust targets（已存在则跳过）"
for entry in "${TARGETS[@]}"; do
  IFS=: read -r _tool target _out <<< "$entry"
  rustup target add "$target" 2>/dev/null || true
done

echo "==> release 编译 -> npm/dist/"
for entry in "${TARGETS[@]}"; do
  IFS=: read -r tool target out_name <<< "$entry"
  echo "--- [$tool] $target -> npm/dist/$out_name"
  if [[ "$tool" == cross ]]; then
    cross build --release -p hi --target "$target"
  else
    cargo build --release -p hi --target "$target"
  fi
  src="$CARGO_TARGET_DIR/$target/release/hi"
  [[ "$out_name" == *.exe ]] && src="${src}.exe"
  cp "$src" "$DIST/$out_name"
  chmod +x "$DIST/$out_name" 2>/dev/null || true
  echo "    OK"
done

echo
ls -lh "$DIST"
echo "完成。产物仅在 npm/dist/；发布见 .github/workflows/publish.yml（@zggg/hi → GitHub Packages，@i99/hi → npmjs）"
