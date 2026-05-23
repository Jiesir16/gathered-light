#!/usr/bin/env bash
# scripts/server-bootstrap.sh
#
# Tencent Cloud Debian 12/13 服务器**一次性**环境初始化。
# 装好：build-essential / Rust 1.85+ / Node 20 / Docker / nginx / certbot / 防火墙
#
# 用法（在服务器上以非 root 用户跑，例如 gathered）：
#   bash scripts/server-bootstrap.sh
#
# 跑完之后再跑 scripts/server-build.sh 编译产物 + 部署。

set -euo pipefail

if [ "$(id -u)" -eq 0 ]; then
  echo "❌ 不要用 root 跑。先 adduser 一个普通用户（例如 gathered），加入 sudo + docker 组后再跑。"
  exit 1
fi

ARCH="$(uname -m)"
echo "─── 检测：$(lsb_release -ds 2>/dev/null || cat /etc/debian_version) on $ARCH ───"
echo ""

# ──────────────────────────────────────────────────────────────────────
# 1. 系统基础包
# ──────────────────────────────────────────────────────────────────────
echo "── [1/6] apt 基础包 ──"
sudo apt update
sudo apt -y install \
  build-essential pkg-config libssl-dev \
  curl wget git jq ufw ca-certificates gnupg \
  nginx certbot python3-certbot-nginx \
  postgresql-client

# ──────────────────────────────────────────────────────────────────────
# 2. Docker（official one-liner）
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [2/6] Docker ──"
if ! command -v docker > /dev/null; then
  curl -fsSL https://get.docker.com | sudo sh
  sudo usermod -aG docker "$USER"
  echo "  ✅ Docker installed; **退出当前 SSH 重连一次让 docker 组生效**"
else
  echo "  ✅ Docker already installed: $(docker --version)"
fi

# ──────────────────────────────────────────────────────────────────────
# 3. Rust toolchain
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [3/6] Rust ──"
if ! command -v cargo > /dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal
  . "$HOME/.cargo/env"
fi
echo "  ✅ $(rustc --version)"

# ──────────────────────────────────────────────────────────────────────
# 4. Node 20 + npm
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [4/6] Node 20 ──"
if ! command -v node > /dev/null || [ "$(node -v | cut -d. -f1 | tr -d v)" -lt 20 ]; then
  curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
  sudo apt -y install nodejs
fi
echo "  ✅ Node $(node -v) / npm $(npm -v)"

# ──────────────────────────────────────────────────────────────────────
# 5. 防火墙
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [5/6] ufw ──"
if ! sudo ufw status | grep -q "Status: active"; then
  sudo ufw allow OpenSSH
  sudo ufw allow 'Nginx Full'
  sudo ufw --force enable
fi
sudo ufw status numbered

# ──────────────────────────────────────────────────────────────────────
# 6. 部署目录
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [6/6] 部署目录 ──"
sudo mkdir -p /opt/gathered-light
sudo chown "$USER:$USER" /opt/gathered-light
echo "  ✅ /opt/gathered-light owned by $USER"

# ──────────────────────────────────────────────────────────────────────
# 完成
# ──────────────────────────────────────────────────────────────────────
cat <<EOF

══════════════════════════════════════════════════════════════════════
✅ bootstrap 完成。下一步：

  1) 如果 Docker 是这次新装的，**先 exit 重新 SSH** 让 docker 组生效

  2) 拉代码：
       cd /opt/gathered-light
       git clone https://github.com/Jiesir16/gathered-light.git src
       cd src

  3) 编译 + 部署：
       bash scripts/server-build.sh

  4) 配置 .env（参考 .env.example + 腾讯云 COS 凭据）：
       cp .env.example /opt/gathered-light/.env
       chmod 600 /opt/gathered-light/.env
       \$EDITOR /opt/gathered-light/.env

  5) 起容器 + 跑迁移 + seed admin（详见 scripts/server-build.sh 末尾提示）

══════════════════════════════════════════════════════════════════════
EOF
