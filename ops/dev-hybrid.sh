#!/usr/bin/env bash
set -euo pipefail

SERVICE="${1:-}" 

if [[ -z "${SERVICE}" ]]; then
  echo "Usage: scripts/dev-hybrid.sh <service>"
  echo "Known services: logline-engine, logline-timeline"
  exit 1
fi

# Start shared infrastructure in the background.
docker compose up -d postgres redis logline-rules logline-timeline >/dev/null

echo "Using local URLs:"
echo "  Postgres: postgres://logline:password@localhost:5432/logline"
echo "  Redis: redis://localhost:6379"

export DATABASE_URL="postgres://logline:password@localhost:5432/logline"
export TIMELINE_DATABASE_URL="${DATABASE_URL}"
export REDIS_URL="redis://localhost:6379"
export RULES_URL="http://127.0.0.1:8081"
export TIMELINE_WS_URL="ws://127.0.0.1:8082/ws/service"

case "${SERVICE}" in
  logline-engine)
    cargo run -p logline-engine
    ;;
  logline-timeline)
    cargo run -p logline-timeline
    ;;
  *)
    echo "Unknown service: ${SERVICE}" >&2
    exit 1
    ;;
esac
