#!/bin/sh
# One-time setup: point git at the repo's shared hooks.
# Run from the repo root: ./scripts/setup-hooks.sh
set -e
cd "$(git rev-parse --show-toplevel)"
chmod +x .githooks/*
git config core.hooksPath .githooks
echo "OK: Git hooks installed (core.hooksPath = .githooks)"
echo "  pre-commit : cargo fmt check + large-file guard"
echo "  commit-msg : Conventional Commits check"
