#!/bin/bash
# Dashboard Setup & Troubleshooting Guide
# Ethereum Uniswap V2 Price Indexer

set -e

WORKSPACE="/Users/gaveeshjain/Documents/VScode/eth_uniswap_alloy"
cd "$WORKSPACE"

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  Ethereum Uniswap V2 Price Indexer - Quick Start              ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# Function to kill existing processes
cleanup() {
    echo "🧹 Cleaning up existing processes..."
    pkill -f "cargo run.*api" || true
    pkill -f "cargo run.*watch" || true
    sleep 2
    lsof -i :3000 2>/dev/null | grep -v COMMAND | awk '{print $2}' | xargs kill -9 2>/dev/null || true
}

# Function to check port
check_port() {
    if lsof -i :3000 2>/dev/null | grep -q "LISTEN"; then
        return 0
    else
        return 1
    fi
}

# Parse arguments
case "${1:-help}" in
    api)
        echo "▶️  Starting API server on port 3000..."
        cleanup
        sleep 1
        RUST_LOG=info cargo run -- api &
        API_PID=$!
        sleep 5
        
        if check_port; then
            echo "✅ API server started (PID: $API_PID)"
            echo "📊 Dashboard: http://localhost:3000"
            echo "📚 OpenAPI Docs: http://localhost:3000/swagger-ui/"
            wait $API_PID
        else
            echo "❌ Failed to start API server"
            exit 1
        fi
        ;;
    
    watch)
        echo "▶️  Starting price indexer in watch mode..."
        echo "📌 This will continuously index WETH/USDT prices from Ethereum"
        RUST_LOG=info cargo run -- watch
        ;;
    
    both)
        echo "▶️  Starting API server and indexer..."
        cleanup
        sleep 1
        
        # Start API in background
        echo "🚀 Starting API Server..."
        RUST_LOG=info cargo run -- api &
        API_PID=$!
        sleep 5
        
        if ! check_port; then
            echo "❌ Failed to start API server"
            exit 1
        fi
        echo "✅ API Server started (PID: $API_PID)"
        echo "📊 Dashboard: http://localhost:3000"
        
        # Start indexer in background
        echo ""
        echo "🚀 Starting Price Indexer..."
        RUST_LOG=info cargo run -- watch &
        WATCH_PID=$!
        echo "✅ Indexer started (PID: $WATCH_PID)"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "✨ System running!"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "📊 Dashboard:        http://localhost:3000"
        echo "📚 API Docs:         http://localhost:3000/swagger-ui/"
        echo "🔌 WebSocket:        ws://localhost:3000/api/v1/stream/WETH-USDT"
        echo ""
        echo "Press Ctrl+C to stop services"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        
        # Wait for both processes
        trap "kill $API_PID $WATCH_PID 2>/dev/null; exit" INT TERM
        wait
        ;;
    
    test|integration)
        echo "🧪 Running integration tests..."
        echo ""
        
        cleanup
        sleep 1
        
        # Start API
        echo "Starting API server..."
        RUST_LOG=info cargo run -- api &
        API_PID=$!
        sleep 5
        
        if ! check_port; then
            echo "❌ API server failed to start"
            kill $API_PID 2>/dev/null || true
            exit 1
        fi
        
        echo "✅ API server ready"
        echo ""
        echo "Testing endpoints:"
        echo ""
        
        # Test health
        echo "1️⃣  Health Check:"
        curl -s http://localhost:3000/api/v1/health | jq -r '.status, .database_status, .websocket_status'
        echo ""
        
        # Test pools
        echo "2️⃣  Pools List:"
        curl -s http://localhost:3000/api/v1/pools | jq '.[] | {name, total_events}'
        echo ""
        
        # Test current price
        echo "3️⃣  Current Price:"
        curl -s http://localhost:3000/api/v1/price/current/WETH-USDT | jq '{price, change_24h, timestamp}'
        echo ""
        
        # Test stats
        echo "4️⃣  24H Statistics:"
        curl -s http://localhost:3000/api/v1/stats/WETH-USDT | jq '{current_price, high, low, average, change_percent}'
        echo ""
        
        # Test events
        echo "5️⃣  Recent Events:"
        curl -s http://localhost:3000/api/v1/events/WETH-USDT | jq '.events | length'
        echo "   events available"
        echo ""
        
        # Cleanup
        kill $API_PID 2>/dev/null || true
        echo "✅ Integration tests complete"
        ;;
    
    db|database)
        echo "📊 Database Browser"
        sqlite3 indexer.db
        ;;
    
    clean|reset)
        echo "🧹 Cleaning..."
        cleanup
        rm -f indexer.db indexer.db-shm indexer.db-wal state.json
        echo "✅ Database and state files removed"
        cargo run -- api &
        API_PID=$!
        sleep 5
        echo "✅ Fresh API server started (the pool will be re-seeded automatically)"
        echo "📊 Dashboard: http://localhost:3000"
        wait $API_PID
        ;;
    
    check-deps)
        echo "🔍 Checking dependencies..."
        echo ""
        echo "Rust:"
        rustc --version
        cargo --version
        echo ""
        echo "System:"
        echo "macOS $(sw_vers -productVersion)"
        echo ""
        echo "Required tools:"
        command -v sqlite3 > /dev/null && echo "✅ sqlite3" || echo "❌ sqlite3"
        command -v curl > /dev/null && echo "✅ curl" || echo "❌ curl"
        command -v jq > /dev/null && echo "✅ jq" || echo "❌ jq"
        echo ""
        ;;
    
    help|*)
        cat << 'EOF'

USAGE: ./run.sh [COMMAND]

COMMANDS:

  api          Start the HTTP/WebSocket API server (port 3000)
               Opens dashboard at http://localhost:3000

  watch        Start the price indexer in continuous watch mode
               Requires API server to be running in another terminal
               Indexes real Ethereum WETH/USDT prices

  both         Run API server and indexer together
               Best for full functionality

  test         Run integration tests to verify everything works
               Starts tests then shuts down automatically

  db           Open SQLite database browser
               Explore database contents interactively

  clean        Reset database and start fresh
               Removes indexer.db, state.json, and WAL files

  check-deps   Verify system dependencies are installed

  help         Show this help message

QUICK START:

  # Full system with API + Indexer (recommended)
  ./run.sh both

  # Or run separately in two terminals:
  Terminal 1: ./run.sh api
  Terminal 2: ./run.sh watch

  # Then open: http://localhost:3000

TROUBLESHOOTING:

  Dashboard shows blank prices?
  → The indexer needs to populate data. Run `./run.sh both`
  → Or insert test data and check API responses

  Can't connect to RPC?
  → Check ETHEREUM_RPC_URL in .env file

  Port 3000 already in use?
  → Run: ./run.sh clean
  → Or kill manually: lsof -i :3000 | kill -9 $(awk 'NR>1 {print $2}')

  What endpoints does the API expose?
  → Visit http://localhost:3000/swagger-ui/ for interactive API docs

EOF
        ;;
esac
