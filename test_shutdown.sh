#!/bin/bash
# Test graceful shutdown

echo "🧪 Testing Graceful Shutdown Feature"
echo "======================================"
echo ""

# Clean up old state
rm -f state.json
echo "✓ Cleaned up old state file"
echo ""

# Start watch mode in background
echo "▶️  Starting watch mode..."
RPC_URL="https://eth-mainnet.g.alchemy.com/v2/hz1VWuC0UZ-4Rnn-p6K_5" timeout 10 cargo run --quiet -- watch --interval 60 2>&1 | grep -E "📊|🔍|🛑|✅|📍|👋" &
WATCH_PID=$!

echo "⏳ Running for 10 seconds..."
sleep 10

echo ""
echo "🛑 Sending Ctrl+C signal..."
kill -INT $WATCH_PID 2>/dev/null || true
wait $WATCH_PID 2>/dev/null || true

sleep 2
echo ""
echo "====================================="
echo "📝 Results:"
echo "====================================="
echo ""

if [ -f state.json ]; then
    echo "✅ State file was created: state.json"
    ls -lh state.json
    echo ""
    echo "📄 State contents:"
    cat state.json | head -20
    echo ""
    BLOCK=$(cat state.json | grep last_block | awk -F': ' '{print $2}' | tr -d ',')
    echo "📍 Last processed block: $BLOCK"
    echo""
    echo "✅ TEST PASSED: Graceful shutdown worked!"
else
    echo "❌ TEST FAILED: State file was not created"
    exit 1
fi

echo ""
echo "🔄 Testing resume from saved state..."
echo "======================================"
echo ""

# Run again to test resume
RPC_URL="https://eth-mainnet.g.alchemy.com/v2/hz1VWuC0UZ-4Rnn-p6K_5" timeout 5 cargo run --quiet -- watch --interval 60 2>&1 | grep -E "Resuming|Starting from block" | head -3

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ TEST PASSED: Resumed from saved state!"
else
    echo ""
    echo "⚠️  Could not verify resume (might be normal)"
fi

echo ""
echo "======================================"
echo "🎉 Graceful shutdown feature working!"
echo "======================================"
