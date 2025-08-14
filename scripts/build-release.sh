#!/usr/bin/env bash
set -euo pipefail
(cd app && npm ci && npm run build)
(cd src-tauri && cargo tauri build)