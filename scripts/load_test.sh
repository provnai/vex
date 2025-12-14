#!/bin/bash
# VEX Load Test Script
# Requires: wrk (brew install wrk / apt install wrk)

set -e

BASE_URL="${VEX_API_URL:-http://localhost:3000}"
DURATION="${DURATION:-30s}"
THREADS="${THREADS:-4}"
CONNECTIONS="${CONNECTIONS:-100}"

echo "==================================="
echo "VEX API Load Test"
echo "==================================="
echo "Target: $BASE_URL"
echo "Duration: $DURATION"
echo "Threads: $THREADS"
echo "Connections: $CONNECTIONS"
echo ""

# Check if wrk is installed
if ! command -v wrk &> /dev/null; then
    echo "Error: wrk is not installed"
    echo "Install with: brew install wrk (macOS) or apt install wrk (Linux)"
    exit 1
fi

# Test 1: Health endpoint (no auth)
echo "--- Test 1: Health Endpoint ---"
wrk -t$THREADS -c$CONNECTIONS -d$DURATION "$BASE_URL/health"
echo ""

# Test 2: Detailed health endpoint
echo "--- Test 2: Detailed Health Endpoint ---"
wrk -t$THREADS -c$CONNECTIONS -d$DURATION "$BASE_URL/health/detailed"
echo ""

# Test 3: Agent creation (requires auth token)
if [ -n "$VEX_TOKEN" ]; then
    echo "--- Test 3: Agent Creation (with auth) ---"
    wrk -t$THREADS -c$CONNECTIONS -d$DURATION \
        -H "Authorization: Bearer $VEX_TOKEN" \
        -H "Content-Type: application/json" \
        -s <(cat <<'EOF'
wrk.method = "POST"
wrk.body = '{"name": "LoadTestAgent", "role": "Tester"}'
EOF
) \
        "$BASE_URL/api/v1/agents"
else
    echo "--- Test 3: Skipped (set VEX_TOKEN for auth tests) ---"
fi

echo ""
echo "==================================="
echo "Load Test Complete"
echo "==================================="
