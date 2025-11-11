# S3 Setup for Hyperliquid L2 Data

The Hyperliquid archive bucket (`s3://hyperliquid-archive`) appears to require `--request-payer requester` 
(based on the Python download script pattern). This means:

1. **AWS credentials are required** - requests must be authenticated
2. **You pay for data transfer** (~$0.09/GB) - you pay AWS for data transfer costs
3. **The `request-payer=requester` header must be included** in all requests

**Note**: This is inferred from the Python script pattern. If you encounter issues, you may need to:
- Configure AWS credentials
- Ensure the `request-payer=requester` header is sent (already handled in the code)

## Setup AWS Credentials

### Option 1: AWS CLI (Recommended)

```bash
# Install AWS CLI if not already installed
# macOS: brew install awscli
# Linux: sudo apt-get install awscli

# Configure credentials
aws configure
# Enter your AWS Access Key ID
# Enter your AWS Secret Access Key
# Default region: us-east-1 (or any)
# Default output format: json
```

### Option 2: Environment Variables

```bash
export AWS_ACCESS_KEY_ID="your-access-key-id"
export AWS_SECRET_ACCESS_KEY="your-secret-access-key"
export AWS_DEFAULT_REGION="us-east-1"
```

### Option 3: Credentials File

Create `~/.aws/credentials`:

```ini
[default]
aws_access_key_id = your-access-key-id
aws_secret_access_key = your-secret-access-key
```

## Test S3 Access

```bash
# Test if you can list the bucket (requires AWS credentials)
aws s3 ls s3://hyperliquid-archive/market_data/20230916/9/ --request-payer requester

# Or test download directly
aws s3 cp s3://hyperliquid-archive/market_data/20230916/9/l2Book/BTC.lz4 ./test.lz4 --request-payer requester
```

## Cost Considerations

- **Data transfer costs**: You pay AWS for downloading data
- **Typical cost**: ~$0.09 per GB for data transfer out
- **Example**: 1 hour of L2 data for BTC might be ~50-100 MB

## Alternative: Use Existing Data

If you already have L2 data files downloaded (e.g., from the Python script mentioned in the README), you can place them in `data/s3/{COIN}/` directory and use `build-events` directly:

```bash
# If you have existing .lz4 files
./target/release/hl-backtest ingest build-events \
  --coin BTC \
  --input data/s3 \
  --out data/events
```

## Troubleshooting

If downloads fail:

1. **Check AWS credentials**: `aws sts get-caller-identity`
2. **Check bucket access**: `aws s3 ls s3://hyperliquid-archive/ --request-payer requester`
3. **Check date availability**: Not all dates/hours may have data
4. **Check network**: Ensure you can reach AWS S3

