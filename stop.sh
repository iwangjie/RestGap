#!/bin/bash
# 息间（RestGap）停止脚本

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_SLUG="restgap"
LEGACY_SLUG="restp"
APP_LABEL="$APP_SLUG"

PID_FILE="$SCRIPT_DIR/$APP_SLUG.pid"
LEGACY_PID_FILE="$SCRIPT_DIR/$LEGACY_SLUG.pid"

if [ ! -f "$PID_FILE" ]; then
    if [ -f "$LEGACY_PID_FILE" ]; then
        echo "⚠️  未找到 $APP_SLUG PID 文件，但发现旧版本 PID 文件：$LEGACY_PID_FILE"
        PID_FILE="$LEGACY_PID_FILE"
        APP_LABEL="$LEGACY_SLUG"
    else
        echo "⚠️  未找到 PID 文件，$APP_SLUG 可能未运行"
    fi

    # 尝试查找进程
    PIDS=$(pgrep -f "target/release/$APP_SLUG" || true)
    LEGACY_PIDS=$(pgrep -f "target/release/$LEGACY_SLUG" || true)
    PIDS="${PIDS}${PIDS:+ }${LEGACY_PIDS}"
    if [ -n "$PIDS" ]; then
        echo "🔍 找到可能的进程: $PIDS"
        echo -n "是否终止这些进程? [y/N] "
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            kill $PIDS
            echo "✅ 已发送终止信号"
        fi
    fi
    exit 0
fi

PID=$(cat "$PID_FILE")

if ! ps -p "$PID" > /dev/null 2>&1; then
    echo "⚠️  进程 $PID 不存在，清理 PID 文件"
    rm -f "$PID_FILE"
    exit 0
fi

echo "🛑 停止 $APP_LABEL (PID: $PID)..."
kill "$PID"

# 等待进程结束
for i in {1..5}; do
    if ! ps -p "$PID" > /dev/null 2>&1; then
        echo "✅ $APP_LABEL 已停止"
        rm -f "$PID_FILE"
        exit 0
    fi
    sleep 1
done

# 如果还没停止，强制终止
if ps -p "$PID" > /dev/null 2>&1; then
    echo "⚠️  正常终止超时，强制停止..."
    kill -9 "$PID"
    sleep 1
    if ! ps -p "$PID" > /dev/null 2>&1; then
        echo "✅ $APP_LABEL 已强制停止"
    else
        echo "❌ 无法停止进程 $PID"
        exit 1
    fi
fi

rm -f "$PID_FILE"
