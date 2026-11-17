#!/usr/bin/env bash
#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
# Fetch exact locked Rust sources, including the optional MERLIN engine.
# Keep downloads in the existing project-local Cargo cache; do not build or install.
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
DRY_RUN=0
case "${1:-}" in
  '') ;;
  --dry-run) DRY_RUN=1 ;;
  -h|--help)
    printf 'Usage: scripts/fetch-development-dependencies.sh [--dry-run]\nFetch locked workspace/MERLIN and Qt sources without building.\n'
    exit 0
    ;;
  *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2 ;;
esac
if [[ $# -gt 1 ]]; then
  printf 'Choose no arguments or --dry-run.\n' >&2
  exit 2
fi
MANIFESTS=("${PROJECT_DIR}/Cargo.toml" "${PROJECT_DIR}/apps/qt/Cargo.toml")
for manifest in "${MANIFESTS[@]}"; do
  ARGUMENTS=(fetch --manifest-path "$manifest" --locked)
  if [[ "$DRY_RUN" == 1 ]]; then
    printf 'Would fetch locked dependencies: %s\n' "$manifest"
  else
    printf 'Fetching locked dependencies: %s\n' "$manifest"
    bash "${SCRIPT_DIR}/cargo-local.sh" "${ARGUMENTS[@]}"
  fi
done
