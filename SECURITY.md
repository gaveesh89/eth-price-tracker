# Security & Secrets Management

## ✅ Current Security Status

### Verified Safe:
- ✅ `.env` file is **properly ignored** by git
- ✅ No `.env` found in git history
- ✅ No actual API keys in repository
- ✅ No hardcoded secrets in source code
- ✅ `.env.example` contains **only placeholders** (YOUR_API_KEY_HERE)
- ✅ All documentation uses example values only

### Protected Files:
```
.env                    # ← Your secrets (NOT in git) ✓
.env.example           # ← Template (safe to commit) ✓
state.json             # ← Runtime state (ignored)
*.db, *.sqlite        # ← Database files (ignored)
```

### Repository Files Safe for Sharing:
- ✅ ARCHITECTURE.md
- ✅ README.md
- ✅ USAGE.md
- ✅ CONTRIBUTING.md
- ✅ All source code (src/*)
- ✅ .env.example (template only)

## Security Best Practices Implemented

### 1. Environment Secrets
```bash
# ✓ Use environment variables (not files or code)
export RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
cargo run -- price

# ✓ Use .env file locally (ignored by git)
cp .env.example .env
# Edit .env with your real key
cargo run -- price
```

### 2. Never Commit Secrets
```bash
# ✓ What's safe (committed to git)
.env.example          # Template with placeholders

# ✗ What's NOT committed (in .gitignore)
.env                  # Your actual secrets
config.json           # Any config with API keys
state.json            # Runtime state
```

### 3. Pre-commit Safety Checks

Add this Git pre-commit hook to prevent accidental secret commits:

```bash
#!/bin/bash
# Save as: .git/hooks/pre-commit
# Make executable: chmod +x .git/hooks/pre-commit

echo "🔒 Running security pre-commit checks..."

# Check for common secret patterns
PATTERNS=(
    "ALCHEMY_API_KEY\s*=\s*[a-zA-Z0-9]"
    "RPC_URL\s*=\s*https.*g.alchemy.com/v2/[a-zA-Z0-9]"
    "api[_-]?key\s*=\s*[a-zA-Z0-9]"
    "secret\s*=\s*[a-zA-Z0-9]"
    "password\s*=\s*[a-zA-Z0-9]"
)

# Staged files
STAGED=$(git diff --cached --name-only)

for file in $STAGED; do
    for pattern in "${PATTERNS[@]}"; do
        if git diff --cached "$file" | grep -iE "$pattern" > /dev/null; then
            echo "❌ ERROR: Possible secret found in $file"
            echo "   Pattern: $pattern"
            echo "   This file would expose secrets!"
            echo ""
            echo "💡 Tips:"
            echo "   1. Remove the secret from $file"
            echo "   2. Use environment variables instead"
            echo "   3. Add the file to .gitignore if needed"
            echo ""
            exit 1
        fi
    done
done

# Check if .env is being staged (should never be!)
if git diff --cached --name-only | grep -E "^\.env$"; then
    echo "❌ ERROR: Attempting to commit .env file!"
    echo "   This file contains your actual API keys."
    echo "   .env should NEVER be committed to git."
    echo ""
    echo "💡 To fix:"
    echo "   git rm --cached .env"
    echo "   git commit --amend"
    echo ""
    exit 1
fi

echo "✅ No secrets detected. Safe to commit!"
exit 0
```

## Installation

```bash
# Create the hook directory if it doesn't exist
mkdir -p .git/hooks

# Copy the security check
curl https://path-to-hook > .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## If You Accidentally Committed Secrets

1. **Immediately rotate your API key** - Generate a new one on Alchemy
2. **Remove from git history:**
   ```bash
   git filter-branch --tree-filter 'rm -f .env' HEAD
   ```
3. **Force push** (if it's your own repo):
   ```bash
   git push --force origin master
   ```

## Automated Secret Scanning

You can use these tools to scan for secrets:

```bash
# Using git-secrets (install first)
git secrets --install
git secrets --register-aws

# Using truffleHog (pip install truffleog)
truffleog filesystem . --only-verified

# Using detect-secrets (pip install detect-secrets)
detect-secrets scan --baseline .secrets.baseline
```

## Environment Variables Reference

| Variable | Type | Required | Where to Set | Example |
|----------|------|----------|--------------|---------|
| `RPC_URL` | String | Yes | `.env` or export | `https://eth-mainnet.g.alchemy.com/v2/...` |
| `RUST_LOG` | String | No | `.env` or export | `info`, `debug` |
| `STATE_FILE` | String | No | `.env` or export | `./state.json` |
| `POLL_INTERVAL_SECS` | Number | No | `.env` or export | `12` |
| `BATCH_SIZE` | Number | No | `.env` or export | `1000` |

## Verification Checklist

Run these to verify security:

```bash
# Check .env is ignored
git check-ignore .env
# Expected: .env

# Check no .env in git
git ls-tree -r HEAD | grep "\.env$"
# Expected: (no output)

# Check for secrets in history
git log --all -S "ALCHEMY_API_KEY" --oneline
# Expected: (no output)

# Check status
git status
# Expected: .env not listed
```

## Questions?

See [CONTRIBUTING.md](CONTRIBUTING.md) for security contribution guidelines.
