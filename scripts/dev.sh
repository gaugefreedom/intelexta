#!/usr/bin/env bash
set -euo pipefail
(cd app && npm run dev) &
(cd src-tauri && cargo tauri dev)
wait