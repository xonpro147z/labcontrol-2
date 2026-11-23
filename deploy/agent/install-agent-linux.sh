#!/bin/bash
# Cài LabControl cho máy học sinh Linux Mint / Ubuntu (phiên X11):
#   sudo bash install-agent-linux.sh [mật-khẩu-phòng] [tên-phòng] [ip-máy-giáo-viên]
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "Hãy chạy bằng sudo"; exit 1; }
PW="${1:-}"
[ -n "$PW" ] || read -rp "Mật khẩu phòng: " PW
ROOM="${2:-}"
TIP="${3:-}"
DIR="$(cd "$(dirname "$0")" && pwd)"
"$DIR/labcontrol-agent" install --password "$PW" --room "$ROOM" --teacher-ip "$TIP"
echo "Xong. Đăng xuất rồi đăng nhập lại; máy sẽ tự hiện trên màn hình của giáo viên."
