#!/bin/bash
# 息间（RestGap）后台启动脚本

# 脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_SLUG="restgap"
LEGACY_SLUG="restp"

BINARY="$SCRIPT_DIR/target/release/$APP_SLUG"
LOG_FILE="$SCRIPT_DIR/$APP_SLUG.log"
PID_FILE="$SCRIPT_DIR/$APP_SLUG.pid"
LEGACY_PID_FILE="$SCRIPT_DIR/$LEGACY_SLUG.pid"

# 检查是否已在运行
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if ps -p "$OLD_PID" > /dev/null 2>&1; then
        echo "⚠️  $APP_SLUG 已经在运行 (PID: $OLD_PID)"
        echo "如需重启，请先运行 ./stop.sh"
        exit 1
    else
        # PID 文件存在但进程不在，清理旧 PID 文件
        rm -f "$PID_FILE"
    fi
fi

# 兼容旧版本 pid 文件
if [ -f "$LEGACY_PID_FILE" ]; then
    OLD_PID=$(cat "$LEGACY_PID_FILE")
    if ps -p "$OLD_PID" > /dev/null 2>&1; then
        echo "⚠️  检测到旧版本仍在运行 (PID: $OLD_PID)"
        echo "请先运行 ./stop.sh 停止旧版本后再启动。"
        exit 1
    else
        rm -f "$LEGACY_PID_FILE"
    fi
fi

# 检查二进制文件是否存在
if [ ! -f "$BINARY" ]; then
    echo "❌ 未找到 $APP_SLUG 二进制文件，请先编译："
    echo "   cargo build --release"
    exit 1
fi

# 配置说明
echo "⚙️  配置：请在菜单栏里点击「配置」修改（每 N 分钟休息 N 秒）。"

# 后台启动（使用 nohup 确保终端关闭后继续运行）
echo ""
echo "🚀 启动 $APP_SLUG..."
nohup "$BINARY" >> "$LOG_FILE" 2>&1 &
APP_PID=$!

# 保存 PID
echo "$APP_PID" > "$PID_FILE"

# 等待一下确认启动成功
sleep 1
if ps -p "$APP_PID" > /dev/null 2>&1; then
    echo "✅ $APP_SLUG 已在后台启动 (PID: $APP_PID)"
    echo "📝 日志文件: $LOG_FILE"
    echo ""
    echo "💡 提示："
    echo "   - 查看日志: tail -f $LOG_FILE"
    echo "   - 停止服务: ./stop.sh"
    echo "   - 检查状态: ./status.sh"
else
    echo "❌ 启动失败，请查看日志: $LOG_FILE"
    rm -f "$PID_FILE"
    exit 1
fi
