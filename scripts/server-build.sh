#!/usr/bin/env bash
# scripts/server-build.sh
#
# 在服务器上**编译 + 部署**到 /opt/gathered-light/。
# 假设 server-bootstrap.sh 已跑过（Rust / Node / Docker 都装好）。
#
# 用法：
#   cd /opt/gathered-light/src   # 你 git clone 的位置
#   bash scripts/server-build.sh                # 全编 + 拷过去
#   bash scripts/server-build.sh --backend-only # 只重编后端
#   bash scripts/server-build.sh --no-restart   # 编完不重启 systemd
#
# 退出码：0 成功；1 编译失败；2 拷贝失败

set -euo pipefail

BACKEND_ONLY=0
NO_RESTART=0
for arg in "$@"; do
  case "$arg" in
    --backend-only) BACKEND_ONLY=1 ;;
    --no-restart)   NO_RESTART=1 ;;
    *) echo "未知参数：$arg" >&2; exit 1 ;;
  esac
done

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DEPLOY_DIR="/opt/gathered-light"
cd "$REPO_DIR"

# 加载 rustup（如果是 fresh shell）
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

echo "═══ 仓库：$REPO_DIR ═══"
echo "═══ 部署目标：$DEPLOY_DIR ═══"
echo "═══ git HEAD：$(git rev-parse --short HEAD 2>/dev/null || echo 'no git') ═══"
echo ""

# ──────────────────────────────────────────────────────────────────────
# 1. 后端：4 个二进制（cms-api + seed_admin + seed_demo + migrations）
# ──────────────────────────────────────────────────────────────────────
echo "── [1/4] cargo build --release（首次 5-8 分钟，增量秒级） ──"
cargo build --release -p cms-api
cargo build --release -p cms-api --bin seed_admin
cargo build --release -p cms-api --bin seed_demo
cargo build --release -p migrations

echo ""
echo "── 产物 ──"
ls -lh target/release/{cms-api,seed_admin,seed_demo,migrations}

# ──────────────────────────────────────────────────────────────────────
# 2. 前端打包（除非 --backend-only）
# ──────────────────────────────────────────────────────────────────────
if [ "$BACKEND_ONLY" = "0" ]; then
  echo ""
  echo "── [2/4] frontend 打包 ──"
  cd frontend
  if [ ! -d node_modules ]; then
    echo "  首次 npm install..."
    npm install
  fi
  npm run build
  cd "$REPO_DIR"
  echo "  ✅ frontend/dist 生成"
fi

# ──────────────────────────────────────────────────────────────────────
# 3. 拷贝到 /opt/gathered-light
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [3/4] 拷贝到 $DEPLOY_DIR ──"
mkdir -p "$DEPLOY_DIR/config"
mkdir -p "$DEPLOY_DIR/frontend"

# 原子替换二进制（mv 比 cp + chmod 安全）
for bin in cms-api seed_admin seed_demo migrations; do
  cp -f "target/release/$bin" "$DEPLOY_DIR/${bin}.new"
  chmod +x "$DEPLOY_DIR/${bin}.new"
done
for bin in cms-api seed_admin seed_demo migrations; do
  if [ -f "$DEPLOY_DIR/$bin" ]; then
    mv "$DEPLOY_DIR/$bin" "$DEPLOY_DIR/${bin}.old"
  fi
  mv "$DEPLOY_DIR/${bin}.new" "$DEPLOY_DIR/$bin"
done

# 配置（首次拷过去，已存在则保留用户改过的）
if [ ! -f "$DEPLOY_DIR/config/default.toml" ]; then
  cp config/default.toml "$DEPLOY_DIR/config/"
  echo "  ✅ config/default.toml（首次）"
else
  echo "  ⚠️ 保留现有 $DEPLOY_DIR/config/default.toml（如需更新自行 cp）"
fi

# docker-compose 同理
if [ ! -f "$DEPLOY_DIR/docker-compose.yml" ]; then
  cp docker-compose.yml "$DEPLOY_DIR/"
  echo "  ✅ docker-compose.yml（首次）"
fi

# 前端 dist（每次全量替换）
if [ "$BACKEND_ONLY" = "0" ]; then
  rm -rf "$DEPLOY_DIR/frontend/dist"
  cp -r frontend/dist "$DEPLOY_DIR/frontend/"
  echo "  ✅ frontend/dist 更新"
fi

# .env.example 仅参考（用户的真 .env 自己维护）
cp .env.example "$DEPLOY_DIR/.env.example"

echo ""
ls -lh "$DEPLOY_DIR"

# ──────────────────────────────────────────────────────────────────────
# 4. 重启 systemd（如果已配置）
# ──────────────────────────────────────────────────────────────────────
if [ "$NO_RESTART" = "0" ] && systemctl list-unit-files 2>/dev/null | grep -q '^cms-api.service'; then
  echo ""
  echo "── [4/4] 重启 cms-api ──"
  sudo systemctl restart cms-api
  sleep 3
  if curl -fsS http://127.0.0.1:8080/readyz > /dev/null; then
    echo "  ✅ cms-api healthy"
  else
    echo "  ⚠️ /readyz 失败，检查日志：sudo journalctl -u cms-api -n 30"
  fi
fi

# ──────────────────────────────────────────────────────────────────────
# 完成
# ──────────────────────────────────────────────────────────────────────
cat <<EOF

══════════════════════════════════════════════════════════════════════
✅ 编译 + 部署完成

下一步首次部署还需要：

  1) **创建 .env**（包含 PG / Redis / 腾讯云 COS 凭据 + 强 JWT secret）：
       cp $DEPLOY_DIR/.env.example $DEPLOY_DIR/.env
       chmod 600 $DEPLOY_DIR/.env
       \$EDITOR $DEPLOY_DIR/.env

  2) **起 PG + Redis**：
       cd $DEPLOY_DIR
       docker compose --env-file .env up -d postgres redis
       docker compose ps

  3) **跑迁移 + seed 第一个 owner**：
       cd $DEPLOY_DIR
       set -a; source .env; set +a
       ./migrations up
       ./seed_admin owner@yourdomain.com '你的强密码'

  4) **systemd 启 cms-api**（仅首次；之后本脚本自动 restart）：
       sudo tee /etc/systemd/system/cms-api.service > /dev/null <<'UNIT'
[Unit]
Description=gathered-light cms-api
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=\$USER
WorkingDirectory=$DEPLOY_DIR
EnvironmentFile=$DEPLOY_DIR/.env
ExecStart=$DEPLOY_DIR/cms-api
Restart=on-failure
RestartSec=3
MemoryMax=512M

[Install]
WantedBy=multi-user.target
UNIT
       sudo systemctl daemon-reload
       sudo systemctl enable --now cms-api
       sudo journalctl -u cms-api -f

  5) **nginx + HTTPS**（参考 docs/ARCHITECTURE.md 部署章节，或之前 chat 给的 nginx 配置）

══════════════════════════════════════════════════════════════════════
EOF
