#!/bin/bash
# Cài LabControl Agent trên Linux Mint / Ubuntu (phiên X11):
#   sudo bash install-agent-linux.sh LCT-xxxx [TenPhong]
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "Hãy chạy bằng sudo"; exit 1; }
CODE="${1:-}"
[ -n "$CODE" ] || read -rp "Dán mã giáo viên LCT-...: " CODE
ROOM="${2:-}"
DIR="$(cd "$(dirname "$0")" && pwd)"
"$DIR/labcontrol-agent" install --teacher "$CODE" --room "$ROOM"
echo "Xong. Đăng xuất rồi đăng nhập lại để Agent bắt đầu chạy."
