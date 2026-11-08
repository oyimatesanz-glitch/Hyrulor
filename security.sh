#!/bin/bash
set -e
echo "== cargo audit =="
cargo audit
echo "== cargo deny =="
cargo deny check
echo "== semgrep =="
~/.local/bin/semgrep --config=auto src/ --error
