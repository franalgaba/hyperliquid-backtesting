# Security Documentation

This document provides comprehensive security information for the Hyperliquid backtester.

## Security Audit

A comprehensive security audit was performed on the codebase, focusing on:
- Input validation and sanitization
- Path traversal vulnerabilities
- API security
- Dependency security
- Error handling and information disclosure

### Key Findings

1. **Input Validation**: Enhanced validation for all user inputs
2. **Path Traversal**: Added protection against directory traversal attacks
3. **API Security**: Improved HTTPS and certificate validation
4. **Dependency Security**: Reviewed all external dependencies

## Security Fixes Applied

### 1. Path Traversal Protection ✅

**Files Modified**:
- `src/cli.rs` - Enhanced `validate_asset()` function
- `src/ingest/s3.rs` - Added `validate_coin()` function
- `src/perps/funding.rs` - Added `validate_coin_for_api()` function

**Changes**:
- Added validation to prevent path traversal characters (`..`, `/`, `\`)
- Added length validation (1-20 characters)
- Restricted to alphanumeric characters (and underscore for coins)
- Prevents injection of malicious paths into file operations

**Example**:
```rust
fn validate_coin(coin: &str) -> Result<()> {
    // Prevent path traversal
    if coin.contains("..") || coin.contains('/') || coin.contains('\\') {
        anyhow::bail!("Invalid coin name: path traversal detected");
    }
    // ... additional validation
}
```

### 2. Input Validation ✅

**Coin Names**:
- Must be 1-20 characters
- Alphanumeric and underscore only
- No path traversal characters

**Dates**:
- Must be YYYYMMDD format (8 digits)
- Validated before use in file paths

**Timestamps**:
- Range validation (start <= end)
- Maximum range limit (1 year) to prevent DoS
- Validated before API calls

### 3. API Security ✅

**HTTP Client Configuration**:
- Explicit certificate validation (`danger_accept_invalid_certs(false)`)
- 30-second timeout on all requests
- HTTPS required for all external API calls

**Funding API**:
- Coin parameter validation before API calls
- Timestamp range validation
- Error handling without exposing sensitive information

### 4. S3 Access Security ✅

**Validation**:
- Coin name validation before S3 operations
- Date format validation
- Hour range validation (0-23)

**AWS Credentials**:
- Uses standard AWS credential chain
- No hardcoded credentials
- Supports environment variables and IAM roles

## Security Best Practices

### For Developers

1. **Always validate user input** before using it in file operations or API calls
2. **Use the validation functions** provided in the codebase
3. **Never trust external data** - validate and sanitize all inputs
4. **Use HTTPS** for all external API calls
5. **Set timeouts** on all network requests
6. **Handle errors gracefully** without exposing sensitive information

### For Users

1. **Use valid coin names** - alphanumeric only, 1-20 characters
2. **Use valid date formats** - YYYYMMDD for dates
3. **Configure AWS credentials securely** - use IAM roles when possible
4. **Review S3 costs** - large date ranges can incur significant transfer costs

## Reporting Security Issues

If you discover a security vulnerability, please:
1. Do not open a public issue
2. Contact the maintainers directly
3. Provide detailed information about the vulnerability
4. Allow time for the issue to be addressed before public disclosure

## Security Checklist

- [x] Input validation for all user inputs
- [x] Path traversal protection
- [x] API security (HTTPS, certificates, timeouts)
- [x] Dependency security review
- [x] Error handling without information disclosure
- [x] Secure credential management
- [x] Documentation of security practices

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Best Practices](https://rust-lang.github.io/rust-clippy/master/index.html#security)
- [AWS Security Best Practices](https://aws.amazon.com/security/best-practices/)

