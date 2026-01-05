#!/bin/bash
# 息间（RestGap）状态检查脚本

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_SLUG="restgap"
LEGACY_SLUG="restp"
APP_LABEL="$APP_SLUG"

PID_FILE="$SCRIPT_DIR/$APP_SLUG.pid"
LOG_FILE="$SCRIPT_DIR/$APP_SLUG.log"
LEGACY_PID_FILE="$SCRIPT_DIR/$LEGACY_SLUG.pid"
LEGACY_LOG_FILE="$SCRIPT_DIR/$LEGACY_SLUG.log"

if [ ! -f "$PID_FILE" ] && [ -f "$LEGACY_PID_FILE" ]; then
  PID_FILE="$LEGACY_PID_FILE"
  LOG_FILE="$LEGACY_LOG_FILE"
  APP_LABEL="$LEGACY_SLUG"
fi

echo "📊 $APP_LABEL 状态"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")

    if ps -p "$PID" > /dev/null 2>&1; then
        echo "✅ 运行中"
        echo "   PID: $PID"

        # 显示进程信息
        PS_INFO=$(ps -p "$PID" -o etime,rss | tail -n 1)
        UPTIME=$(echo "$PS_INFO" | awk '{print $1}')
        MEM_KB=$(echo "$PS_INFO" | awk '{print $2}')
        MEM_MB=$((MEM_KB / 1024))

        echo "   运行时间: $UPTIME"
        echo "   内存占用: ${MEM_MB} MB"
        echo ""
        echo "⚙️  配置：请在菜单栏里点击「配置」查看/修改。"
    else
        echo "❌ 未运行 (PID 文件存在但进程不在)"
        echo "   过期 PID: $PID"
    fi
else
    echo "❌ 未运行"

    # 检查是否有遗留进程
    PIDS=$(pgrep -f "target/release/$APP_SLUG" || true)
    LEGACY_PIDS=$(pgrep -f "target/release/$LEGACY_SLUG" || true)
    PIDS="${PIDS}${PIDS:+ }${LEGACY_PIDS}"
    if [ -n "$PIDS" ]; then
        echo "   ⚠️  发现可能的遗留进程: $PIDS"
    fi
fi

echo ""
echo "📝 日志文件: $LOG_FILE"
if [ -f "$LOG_FILE" ]; then
    LOG_SIZE=$(du -h "$LOG_FILE" | awk '{print $1}')
    echo "   大小: $LOG_SIZE"

    if [ -s "$LOG_FILE" ]; then
        echo ""
        echo "📄 最近日志 (最后 5 行):"
        tail -n 5 "$LOG_FILE" | sed 's/^/   /'
    fi
else
    echo "   (不存在)"
fi
