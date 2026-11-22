#!/bin/bash
# Đổi địa chỉ kho mã ghi trong dự án (README, Cargo.toml, config Server...), ví dụ khi fork:
#   bash tools/dat-dia-chi-repo.sh https://github.com/ten-ban/labcontrol
set -euo pipefail
NEW="${1:?Cần địa chỉ repo, ví dụ https://github.com/ten-ban/labcontrol}"
OLD="${2:-https://github.com/xonpro147z/labcontrol-2}"
cd "$(dirname "$0")/.."
grep -rlF "$OLD" --exclude-dir=node_modules --exclude-dir=target --exclude-dir=.git --exclude=dat-dia-chi-repo.sh . |
  while read -r f; do sed -i "s#${OLD}#${NEW%/}#g" "$f"; echo "đã sửa $f"; done
