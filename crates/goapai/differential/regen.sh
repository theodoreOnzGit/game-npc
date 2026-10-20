#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# Regenerate tests/fixtures/differential.txt from the ORIGINAL Go goapai.
# Needs git and Go >= 1.24. Fetches upstream at the pinned commit into a
# temporary directory; nothing is installed.
set -euo pipefail

UPSTREAM=https://github.com/akmonengine/goapai
COMMIT=2e86c5ee56d6d10f43ed563214b1aa86be83db2d

here="$(cd "$(dirname "$0")" && pwd)"
out="$here/../tests/fixtures/differential.txt"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

git clone --quiet "$UPSTREAM" "$tmp/goapai"
git -C "$tmp/goapai" checkout --quiet "$COMMIT"
mkdir -p "$tmp/goapai/cmd/differential"
cp "$here/main.go" "$tmp/goapai/cmd/differential/main.go"
# GetPlan prints "no goal available" to stdout; discard it.
(cd "$tmp/goapai" && go run ./cmd/differential "$out" > /dev/null)
echo "wrote $out ($(grep -c '^S ' "$out") scenarios, go: $(go version))"
