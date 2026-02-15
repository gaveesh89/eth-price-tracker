# Chain Reorganization Handling

## Overview

This document describes how the Ethereum Price Tracker detects and handles chain reorganizations (reorgs) to maintain data integrity during blockchain reorganization events.

## What Are Chain Reorganizations?

A chain reorganization occurs when the Ethereum network adopts a different chain of blocks as the canonical chain, invalidating previously accepted blocks. This happens naturally due to network consensus mechanisms.

### Frequency on Ethereum Mainnet

- **1-block reorgs**: 3-10 times per day (common)
- **2-block reorgs**: 1-3 times per week  
- **3-block reorgs**: 1-2 times per month
- **4-7 block reorgs**: Rare, monthly or less
- **Finalized blocks** (64+ confirmations, ~12.8 minutes): Never reorganize

## Implementation

### Architecture

The reorg detection system consists of three main components:

1. **BlockRecord** - Minimal block metadata storage
   - Block number and hash
   - Parent block hash
   - Timestamp

2. **ReorgDetector** - Chain continuity verification
   - Tracks last known block hash
   - Detects parent hash mismatches
   - Binary search for fork point
   - Reorg counter for metrics

3. **State Integration** - Persistence and recovery
   - Block hash storage in state.json
   - Reorg count tracking
   - State invalidation on reorgs
   - Automatic re-indexing

### Detection Algorithm

```rust
// On each new block check:
1. Fetch current block from RPC
2. Compare parent_hash with our last_block_hash
3. If match: Chain is continuous, proceed normally
4. If mismatch: REORG DETECTED
   a. Binary search to find fork point
   b. Invalidate state from fork point forward
   c. Increment reorg counter
   d. Re-index blocks from fork point to current
   e. Resume normal processing
```

### State Schema

The state.json file now includes reorg tracking:

```json
{
  "weth_reserve": "0x...",
  "usdt_reserve": "0x...",
  "last_block": 19234567,
  "last_block_hash": "0xabc123...",
  "reorg_count": 0
}
```

## Usage

### Automatic Reorg Detection

Reorg detection runs automatically in watch mode:

```bash
RPC_URL="your_rpc_url" cargo run -- watch
```

When a reorg is detected, you'll see:

```
⚠️  CHAIN REORGANIZATION DETECTED!
🔀 Fork point: block 19234560
📏 Reorg depth: 7 blocks
🔄 Re-indexing from block 19234560...
```

### Monitoring Reorgs

The reorg count persists across restarts in `state.json`:

```bash
# Check reorg statistics
cat state.json | jq .reorg_count

# Monitor for reorg events in logs
cargo run -- watch 2>&1 | grep "REORG DETECTED"
```

## Recovery Process

### When a Reorg is Detected

1. **Detection**: Parent hash mismatch triggers reorg check
2. **Fork Point**: Binary search finds last valid block
3. **Invalidation**: State rolled back to fork point
4. **Re-indexing**: Blocks re-processed from fork point to current
5. **Counter Update**: Reorg count incremented
6. **Persistence**: Updated state saved to state.json
7. **Resume**: Normal processing continues

### Data Integrity Guarantees

- No duplicate event processing
- No missed events
- Consistent price calculations
- Proper reserve tracking
- State persistence across restarts

## Testing

### Unit Tests

Run reorg detection tests:

```bash
cargo test --test reorg_detection
```

Tests cover:
- Block record tracking
- Detector initialization
- State integration
- Persistence across restarts
- Serialization/deserialization

### Integration Testing

Since Anvil doesn't support creating reorgs, test on live networks:

```bash
# Goerli testnet (frequent reorgs)
RPC_URL="https://goerli.infura.io/v3/YOUR_KEY" cargo run -- watch

# Sepolia testnet  
RPC_URL="https://sepolia.infura.io/v3/YOUR_KEY" cargo run -- watch
```

### Monitoring in Production

Track these metrics:

1. **Reorg Frequency**: Check `reorg_count` in state.json
2. **Reorg Depth**: Monitor log messages for depth statistics
3. **Recovery Time**: Time from detection to resumed processing
4. **Data Consistency**: Verify prices match expected values post-reorg

## Best Practices

### Confirmation Depth

For critical operations, use appropriate confirmation depths:

- **1 confirmation**: Real-time data, accept reorg risk
- **6 confirmations**: ~1 minute, very low reorg risk
- **12 confirmations**: ~2.5 minutes, extremely low risk
- **32 confirmations**: ~6.4 minutes, safe block tag threshold
- **64+ confirmations**: ~12.8 minutes, finalized (no reorgs)

### Finalized Block Tag

For applications requiring absolute certainty:

```rust
// Fetch finalized block (never reorgs)
let finalized = provider.get_block_by_number(
    BlockNumberOrTag::Finalized,
    false
).await?;
```

Use finalized block as confirmation anchor for critical state updates.

### State Backup

Regularly backup state.json to recover from corruption:

```bash
# Backup before major updates
cp state.json state.json.backup

# Restore if needed
cp state.json.backup state.json
```

## Performance Impact

Reorg detection adds minimal overhead:

- **Per-block check**: +1 RPC call (fetch block header)
- **Hash comparison**: <1ms CPU time
- **Normal operation**: No performance impact
- **During reorg**: Additional RPC calls for fork point search and re-indexing

Binary search for fork point: `O(log n)` RPC calls where `n` is reorg depth.

## API Reference

### BlockRecord

```rust
pub struct BlockRecord {
    pub number: u64,
    pub hash: B256,
    pub parent_hash: B256,
    pub timestamp: u64,
}
```

### ReorgDetector

```rust
impl ReorgDetector {
    pub fn new() -> Self;
    pub fn add_block(&mut self, block: BlockRecord);
    pub fn detect_reorg(&mut self, provider: &Provider, block: u64) -> Result<Option<u64>>;
    pub fn reorg_count(&self) -> u64;
}
```

### State Methods

```rust
impl State {
    pub fn last_block_hash(&self) -> Option<B256>;
    pub fn set_block_hash(&mut self, hash: B256);
    pub fn reorg_count(&self) -> u64;
    pub fn increment_reorg_count(&mut self);
    pub fn invalidate_from(&mut self, fork_point: u64);
}
```

## Troubleshooting

### Issue: High Reorg Count

**Symptoms**: reorg_count increases rapidly  
**Possible Causes**:
- Using unstable RPC endpoint  
- Network connectivity issues
- Syncing node that's catching up

**Solutions**:
- Use reliable RPC provider (Infura, Alchemy, Ankr)
- Verify node is fully synced
- Increase polling interval to reduce detection frequency

### Issue: State Corruption After Reorg

**Symptoms**: Invalid reserve values, price mismatches  
**Possible Causes**:
- Re-indexing incomplete
- RPC provider returned inconsistent data

**Solutions**:
```bash
# Delete state and re-index from scratch
rm state.json
cargo run -- watch
```

### Issue: Repeated Reorgs at Same Block

**Symptoms**: Reorg detected multiple times at same block height  
**Possible Causes**:
- Node not fully synced
- Network partition
- RPC endpoint issues

**Solutions**:
- Wait for network stability
- Switch RPC providers
- Use finalized block tag for critical operations

## Future Enhancements

Potential improvements:

1. **Reorg History**: Store last N reorgs for analysis
2. **Prometheus Metrics**: Export reorg stats for monitoring
3. **Configurable Depth**: Set fork point search depth limits
4. **Fast Recovery**: Cache recent blocks to avoid re-fetching
5. **Finalized Anchors**: Only process blocks past finalized tag

## Resources

- [Ethereum Consensus Spec](https://ethereum.github.io/consensus-specs/)
- [Understanding Reorgs](https://ethereum.org/en/developers/docs/consensus-mechanisms/pos/#finality)
- [Block Finality](https://ethereum.org/en/developers/docs/consensus-mechanisms/pos/gasper/)
- [Layr-Labs Chain Indexer](https://github.com/Layr-Labs/go-sidecar) - Reference implementation

## See Also

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture
- [TESTING.md](./TESTING.md) - Testing strategies  
- [RUNNING.md](./RUNNING.md) - Operational guide
- [State Module Documentation](./src/state.rs) - State management
- [Reorg Module Documentation](./src/reorg/) - Implementation details
