# Dashboard Setup & Troubleshooting Guide

## ✨ Quick Answer

**Your dashboard shows blank prices because there's no price data in the database yet.** The system is working correctly, but the indexer hasn't populated any data.

**Solution:** You need to run the price indexer to populate real data from Ethereum.

---

## 🚀 Getting Started (3 Options)

### Option 1: Everything in One Command (Easiest) ⭐

```bash
./run.sh both
```

This starts:
- ✅ API server on `http://localhost:3000`
- ✅ Price indexer continuously fetching real data

Then open your browser: **http://localhost:3000**

---

### Option 2: Run Separately (Two Terminals)

**Terminal 1 - API Server:**
```bash
RUST_LOG=info cargo run -- api
```
- Server starts on `http://localhost:3000`
- Dashboard is immediately accessible (but shows no data yet)

**Terminal 2 - Price Indexer:**
```bash
RUST_LOG=info cargo run -- watch
```
- Continuously indexes WETH/USDT prices from Ethereum
- Data populates into the database
- Dashboard updates in real-time

---

### Option 3: Quick Test Without Real Data

To verify everything works without waiting for real data:

```bash
./run.sh test
```

This:
- ✅ Starts the API server
- ✅ Tests all endpoints
- ✅ Shows API response formats
- ✅ Shuts down automatically

---

## 📊 What You Should See

### When API is Running:
```
✨ System running!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 Dashboard:        http://localhost:3000
📚 API Docs:         http://localhost:3000/swagger-ui/
🔌 WebSocket:        ws://localhost:3000/api/v1/stream/WETH-USDT
```

### When Indexer is Running:
```
2026-02-15T14:30:45.123Z INFO  Processing Sync event
  pool: WETH/USDT
  block: 21000123
  price: $2,345.67
  timestamp: 2026-02-15T14:30:45Z
  reserves: 125.34 WETH, 292,456.78 USDT
```

### Dashboard Updates:
- 💰 **Current Price** updates every few seconds
- 📈 **Price Chart** populates with historical data
- 📊 **Stats Cards** show high/low/average/change
- 📋 **Events Table** shows recent Sync transactions

---

## 🔍 Endpoints Available

Once the API is running:

| Endpoint | Purpose | Example |
|----------|---------|---------|
| `GET /` | Dashboard UI | http://localhost:3000 |
| `GET /swagger-ui/` | Interactive API docs | http://localhost:3000/swagger-ui/ |
| `GET /api/v1/health` | Health check | http://localhost:3000/api/v1/health |
| `GET /api/v1/pools` | List pools | http://localhost:3000/api/v1/pools |
| `GET /api/v1/price/current/WETH-USDT` | Current price | http://localhost:3000/api/v1/price/current/WETH-USDT |
| `GET /api/v1/stats/WETH-USDT` | 24h stats | http://localhost:3000/api/v1/stats/WETH-USDT |
| `GET /api/v1/price/history/WETH-USDT` | Price history | http://localhost:3000/api/v1/price/history/WETH-USDT |
| `GET /api/v1/events/WETH-USDT` | Recent events | http://localhost:3000/api/v1/events/WETH-USDT |
| `WS /api/v1/stream/WETH-USDT` | Real-time updates | ws://localhost:3000/api/v1/stream/WETH-USDT |

---

## ❓ Troubleshooting

### Dashboard opens but shows no prices

**Check:** Is the indexer running?
```bash
# Test API manually
curl http://localhost:3000/api/v1/price/current/WETH-USDT | jq .
```

**Solution:**
- Start the indexer: `./run.sh watch` (in another terminal)
- Or run both together: `./run.sh both`

---

### "Port 3000 already in use"

**Fix:**
```bash
# Option 1: Clean everything
./run.sh clean

# Option 2: Kill manually
lsof -i :3000 | tail -1 | awk '{print $2}' | xargs kill -9
```

---

### "Cannot connect to Ethereum RPC"

**Check .env file:**
```bash
cat .env | grep ETHEREUM_RPC_URL
```

**Should show:**
```
ETHEREUM_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
```

**Fix:** Update the URL in `.env` with a valid RPC endpoint
- Alchemy: https://alchemy.com/
- Infura: https://infura.io/
- QuickNode: https://www.quicknode.com/

---

### Dashboard shows "Disconnected" status

This is a UI display issue while the WebSocket is initializing. The API is working fine - test it:
```bash
curl http://localhost:3000/api/v1/pools | jq .
```

---

### Real prices not showing after 30 seconds

**Check:** 
1. Is the indexer running? You should see logs like:
   ```
   Processing Sync event...
   ```

2. Check database:
   ```bash
   ./run.sh db
   > SELECT COUNT(*) FROM price_points;
   ```
   Should return > 0

3. Check indexer logs for errors

---

## 🛠️ Useful Commands

```bash
# View help
./run.sh help

# Check system dependencies
./run.sh check-deps

# Browse database directly
./run.sh db

# Reset everything (removes database)
./run.sh clean

# Run endpoint tests
./run.sh test

# View logs with timestamps
RUST_LOG=debug,tower_http=trace cargo run -- api
```

---

## 📈 Expected Performance

- **Dashboard load:** < 1 second
- **Price update:** 1-5 seconds (depends on Ethereum block time)
- **Chart data:** Populates as new prices arrive
- **API response time:** < 100ms for most endpoints

---

## 🔧 Architecture

```
┌─────────────────────────────────────────────┐
│        Your Browser (Port 3000)             │
│  ┌──────────────────────────────────────┐   │
│  │   Dashboard UI (Tailwind + Chart.js) │   │
│  └────────────────▲─────────────────────┘   │
│                   │                         │
│                   │ (HTTP + WebSocket)     │
└─────────────────────────────────────────────┘
            │                    ▲
            │                    │
            ▼                    │
┌──────────────────────────────────────────────┐
│      Axum API Server (localhost:3000)        │
│  ┌──────────────────────────────────────┐   │
│  │   GET  /api/v1/price/current/:pool   │   │
│  │   GET  /api/v1/stats/:pool           │   │
│  │   GET  /api/v1/price/history/:pool   │   │
│  │   WS   /api/v1/stream/:pool          │   │
│  └──────────────────────────────────────┘   │
└──────────┬──────────────────────────────────┘
           │
           │ (Queries/Inserts)
           ▼
┌──────────────────────────────────────────────┐
│      SQLite Database (indexer.db)            │
│  ┌──────────────────────────────────────┐   │
│  │   pools table      (pool metadata)    │   │
│  │   price_points     (computed prices)  │   │
│  │   sync_events      (raw blockchain)   │   │
│  │   indexer_state    (progress tracking)│   │
│  └──────────────────────────────────────┘   │
└──────────────────────────────────────────────┘
           ▲
           │
           │ (Continuous Polling)
           │
    ┌──────────────────────────────────┐
    │   Ethereum Indexer (watch mode)  │
    │  - Polls Ethereum RPC API        │
    │  - Decodes Uniswap V2 events     │
    │  - Computes prices               │
    │  - Stores in database            │
    └──────────────────────────────────┘
```

---

## 🎯 Next Steps

1. **Start the system:**
   ```bash
   ./run.sh both
   ```

2. **Open dashboard:**
   - Navigate to http://localhost:3000

3. **Monitor logs:**
   - Watch for "Processing Sync event" messages

4. **Adjust indexing:**
   - Check [USAGE.md](USAGE.md) for advanced configuration
   - Modify block range in [src/cli.rs](src/cli.rs)
   - Change update frequency in polling loop

5. **Deploy to production:**
   - See [RUNNING.md](RUNNING.md) for deployment guide

---

## 📚 Documentation

- [Architecture Overview](ARCHITECTURE.md)
- [Usage Guide](USAGE.md)
- [Setup Instructions](README_SETUP.md)
- [Deployment Guide](RUNNING.md)
- [Security Considerations](SECURITY.md)

---

## 💡 Key Insights

- ✅ The API server works (you can verify with curl)
- ✅ The database is initialized (migrations run automatically)
- ✅ The dashboard renders (you can see the UI)
- ⏳ **You just need to start the indexer to populate data**

The indexer is completely separate from the API server. This lets you:
- Run them independently
- Scale them separately  
- Update one without stopping the other
- Use the API without the indexer (just manually insert data)

---

**🎉 That's it! You now have a fully functional Ethereum price tracking system.**
