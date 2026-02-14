# Setup Instructions

## 1. Get Alchemy API Key

1. Go to https://www.alchemy.com/
2. Sign up for a free account
3. Click "Create New App"
   - Name: `eth-price-tracker`
   - Chain: `Ethereum`
   - Network: `Mainnet`
4. Click on your app name
5. Click "View Key"
6. Copy the **HTTPS URL** (looks like: `https://eth-mainnet.g.alchemy.com/v2/abc123...`)

## 2. Configure Environment

1. Open the `.env` file in your project root
2. Find the line: `RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY_HERE`
3. Replace `YOUR_API_KEY_HERE` with your actual Alchemy API key
4. Save the file

## 3. Verify Configuration

Test your RPC connection:
```bash
curl -X POST https://eth-mainnet.g.alchemy.com/v2/YOUR_ACTUAL_KEY \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_blockNumber",
    "params": [],
    "id": 1
  }'
```

Expected response:
```json
{"jsonrpc":"2.0","id":1,"result":"0x..."}
```

## 4. Security Reminder

⚠️ **NEVER commit your `.env` file to git!**

- `.env` = Your secrets (ignored by git)
- `.env.example` = Template (safe to commit)

If you accidentally commit `.env`:
1. Immediately regenerate your Alchemy API key
2. Remove `.env` from git history
3. Update your `.env` file with the new key
