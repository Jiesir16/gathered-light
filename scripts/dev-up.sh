#!/usr/bin/env bash
# scripts/dev-up.sh
#
# 一键拉起 PG 18 + Redis 7 + MinIO + 跑 PR-1 DoD 验证。
#
# 用法：
#   bash scripts/dev-up.sh           # 标准启动 + DoD
#   bash scripts/dev-up.sh --skip-dod   # 仅起依赖，不跑 cms-api
#   bash scripts/dev-up.sh --reset       # 强制清旧卷重来（PG18 mount 改造时用一次）
#
# 退出码：
#   0  全部 OK（DoD 三步 200/503/200 + 启动日志含 postgres/redis）
#   1  PG 健康检查失败
#   2  DoD 任一步未达预期
#   3  cms-api 启动超时

set -euo pipefail
cd "$(dirname "$0")/.."

SKIP_DOD=0
RESET=0
for arg in "$@"; do
  case "$arg" in
    --skip-dod) SKIP_DOD=1 ;;
    --reset)    RESET=1 ;;
    *) echo "未知参数：$arg" >&2; exit 1 ;;
  esac
done

# ──────────────────────────────────────────────────────────────────────
# 1. 清理（仅 --reset 模式）
# ──────────────────────────────────────────────────────────────────────
if [ "$RESET" = "1" ]; then
  echo "── [1/5] --reset：停掉容器并删 ./data/postgres + ./data/redis + ./data/minio ──"
  docker compose down 2>/dev/null || true
  rm -rf ./data/postgres ./data/redis ./data/minio
  # 顺手清理 compose 早期版本可能留下的命名卷（如果存在）
  docker volume rm -f gathered-light_pg18_data gathered-light_redis_data gathered-light_minio_data 2>/dev/null || true
  echo "  ✅ 本地数据目录已清"
else
  echo "── [1/5] 检查同名容器（不健康提示先 --reset） ──"
  if [ "$(docker inspect gathered-pg --format '{{.State.Status}}' 2>/dev/null || echo none)" = "exited" ]; then
    echo "  ⚠️ gathered-pg 处于 Exited，建议先跑：bash scripts/dev-up.sh --reset"
    echo "  （可能是 PG 18 改了 mount 路径或数据损坏）"
    exit 1
  fi
fi

# ──────────────────────────────────────────────────────────────────────
# 2. 拉起 PG + Redis + MinIO（bind mount 目标目录由 compose 自动 mkdir）
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [2/5] 拉起 postgres + redis + minio ──"
mkdir -p ./data/postgres ./data/redis ./data/minio
docker compose up -d postgres redis minio

# ──────────────────────────────────────────────────────────────────────
# 3. 等 PG healthy
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [3/5] 等 PG healthy（最多 30s） ──"
for i in $(seq 1 30); do
  s=$(docker inspect gathered-pg --format '{{.State.Health.Status}}' 2>/dev/null || echo unknown)
  if [ "$s" = "healthy" ]; then echo "  ✅ pg healthy after ${i}s"; break; fi
  if [ "$i" = "30" ]; then
    echo "  ❌ pg 30s 内未 healthy"
    echo "── 容器日志最后 20 行 ──"
    docker logs --tail 20 gathered-pg
    exit 1
  fi
  sleep 1
done
printf "  redis: "
docker exec gathered-redis redis-cli ping

if [ "$SKIP_DOD" = "1" ]; then
  echo ""
  echo "✅ 依赖已就绪。--skip-dod 跳过 PR-1 DoD。"
  echo "   下一步：cargo run -p cms-api  或  发 PR-2 prompt 给 Codex"
  exit 0
fi

# ──────────────────────────────────────────────────────────────────────
# 4. 编译 + 启动 cms-api，跑 PR-1 DoD（§14.1）
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [4/5] 编译 cms-api（首次含 fred/sqlx/rustls/aws-lc-sys 等大依赖，约 1-2 分钟，增量编译后只要几秒） ──"
if ! cargo build -p cms-api; then
  echo "  ❌ cargo build 失败，请先修编译错误"
  exit 4
fi
echo "  ✅ 编译完成"

echo ""
echo "── 启动 cms-api（端口 18080，不撞前端 dev 8080） ──"
APP_BIND_ADDR=127.0.0.1:18080 RUST_LOG=info \
  ./target/debug/cms-api > /tmp/gathered-cms.log 2>&1 &
PID=$!

cleanup() {
  echo ""
  echo "── 收工：kill cms-api pid=$PID ──"
  kill -INT "$PID" 2>/dev/null || true
  wait "$PID" 2>/dev/null || true
}
trap cleanup EXIT

# 运行时启动（连 PG/Redis、装 tracing），通常 1-2 秒；给 20 秒兜底
for i in $(seq 1 40); do
  if curl -fsS -o /dev/null http://127.0.0.1:18080/healthz 2>/dev/null; then break; fi
  if [ "$i" = "40" ]; then
    echo "  ❌ cms-api 20s 内未起来（编译已过，应是 init 报错）"
    echo "── 启动日志最后 30 行 ──"
    tail -30 /tmp/gathered-cms.log
    exit 3
  fi
  sleep 0.5
done

step1=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18080/readyz)
echo "  step-1 (期望 200): HTTP $step1"

docker compose stop postgres > /dev/null 2>&1
sleep 2
step2=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18080/readyz)
echo "  step-2 (期望 503，PG 已停): HTTP $step2"

docker compose start postgres > /dev/null 2>&1
for i in $(seq 1 30); do
  s=$(docker inspect gathered-pg --format '{{.State.Health.Status}}' 2>/dev/null || echo unknown)
  if [ "$s" = "healthy" ]; then break; fi
  sleep 1
done
step3=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18080/readyz)
echo "  step-3 (期望 200，PG 起回来): HTTP $step3"

# ──────────────────────────────────────────────────────────────────────
# 5. 校验 + 摘要
# ──────────────────────────────────────────────────────────────────────
echo ""
echo "── [5/5] 启动日志关键行（应同时出现 postgres / redis 字样） ──"
grep -E "postgres|redis|tracing initialized" /tmp/gathered-cms.log | head -10

echo ""
if [ "$step1$step2$step3" = "200503200" ]; then
  echo "🎉 PR-1 DoD 全过：200/503/200。下一步可以发 PR-2 prompt 给 Codex 了。"
  exit 0
else
  echo "❌ PR-1 DoD 未达预期：$step1/$step2/$step3（应为 200/503/200）"
  echo "   完整日志：cat /tmp/gathered-cms.log"
  exit 2
fi
