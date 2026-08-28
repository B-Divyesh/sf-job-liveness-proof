#!/bin/sh
set -eu
base_url="${RUN_PROOF_URL:-http://localhost:8080}"
seq 1 100 | xargs -P 20 -I request sh -c 'curl --fail --silent --output /dev/null "$1/health"' _ "$base_url"
echo "100 health requests completed"
