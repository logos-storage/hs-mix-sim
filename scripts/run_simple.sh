#!/usr/bin/env bash
# Settings: scripts/params.sh
set -euo pipefail
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
exec bash "$SCRIPT_DIR/lib/run_model.sh" simple
