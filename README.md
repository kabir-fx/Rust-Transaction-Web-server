# Transaction Service

A secure, reliable REST API for managing financial accounts and transactions, built with Rust and PostgreSQL.

![arch_diagram](/assets/arch_diagram.svg)

## Features

- ✅ **API Key Authentication** - Secure service-to-service authentication
- ✅ **Account Management** - Create and manage business accounts with balance tracking
- ✅ **Atomic Transactions** - Credit, debit, and transfer operations with ACID guarantees
- ✅ **Idempotency** - Safe request retries without duplicate processing
- ✅ **Webhooks** - Real-time transaction notifications with HMAC-SHA256 signatures
- ✅ **Docker Ready** - One-command local setup with Docker Compose
- ✅ **Structured Logging** - Operational visibility with configurable log levels

## Technology Stack

- **Web Framework**: Axum 0.8 (async, type-safe routing)
- **Runtime**: Tokio (async execution)
- **Database**: PostgreSQL 16 (ACID transactions, JSONB support)
- **ORM**: sqlx (compile-time query verification)
- **Containerization**: Docker & Docker Compose

## Quick Start

### Prerequisites

- Docker & Docker Compose
- curl (for testing)

### 1. Start the Service

```bash
# Clone the repository
git clone <repository-url>
cd transaction_rust

# Start database and API server
docker compose up --build
```

The service will be available at http://localhost:3000

### 2. Generate API Key

```bash
# Create an API key
./scripts/create_api_key.sh

# Save the output - this is your API key (shown only once)
export API_KEY="<your_api_key_here>"
```

### 3. Test the API

```bash
# Check health
curl http://localhost:3000/health

# Create an account
curl -X POST http://localhost:3000/api/v1/accounts \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_name": "Primary Account",
    "initial_balance_cents": 100000
  }'

# Save the account ID from the response
export ACCOUNT_ID="<account_id_from_response>"

# Add money (credit)
curl -X POST http://localhost:3000/api/v1/transactions/credit \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "'$ACCOUNT_ID'",
    "amount_cents": 50000,
    "description": "Deposit"
  }'
```

Or use the automated test script:

```bash
./scripts/test_api.sh $API_KEY
```

## Documentation

- **[API Reference](API.md)** - Complete API documentation with examples
- **[Design Specification](DESIGN.md)** - Architecture, security, and trade-offs

## Project Structure

```
src/
├── handlers/          # HTTP request handlers
│   ├── accounts.rs    # Account endpoints
│   ├── transactions.rs # Transaction endpoints
│   ├── webhooks.rs    # Webhook endpoints
│   └── health.rs      # Health check
├── services/          # Business logic layer
│   ├── transaction_service.rs  # Transaction operations
│   └── webhook_service.rs      # Webhook delivery
├── models/            # Data structures
│   ├── account.rs     # Account models
│   ├── transaction.rs # Transaction models
│   ├── webhook.rs     # Webhook models
│   └── api_key.rs     # API key models
├── middleware/        # HTTP middleware
│   └── auth.rs        # Authentication middleware
├── config.rs          # Configuration management
├── db.rs              # Database connection pool
├── error.rs           # Error handling
└── main.rs            # Application entry point

migrations/            # Database migrations (sqlx)
scripts/               # Utility scripts
```

## Environment Configuration

Create a `.env` file or set environment variables:

```bash
# Database connection
DATABASE_URL=postgres://postgres:postgres@localhost:5432/transactions

# Server configuration
SERVER_PORT=3000

# Logging (optional)
RUST_LOG=info  # Options: error, warn, info, debug, trace
```

## API Usage Examples

### Create Account

```bash
curl -X POST http://localhost:3000/api/v1/accounts \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_name": "Checking Account",
    "initial_balance_cents": 100000
  }'
```

### Credit (Add Money)

```bash
curl -X POST http://localhost:3000/api/v1/transactions/credit \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "ACCOUNT_ID",
    "amount_cents": 50000,
    "description": "Deposit",
    "idempotency_key": "deposit-001"
  }'
```

### Debit (Remove Money)

```bash
curl -X POST http://localhost:3000/api/v1/transactions/debit \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "ACCOUNT_ID",
    "amount_cents": 10000,
    "description": "Withdrawal"
  }'
```

### Transfer (Between Accounts)

```bash
curl -X POST http://localhost:3000/api/v1/transactions/transfer \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "from_account_id": "FROM_ACCOUNT_ID",
    "to_account_id": "TO_ACCOUNT_ID",
    "amount_cents": 25000,
    "description": "Payment"
  }'
```

### Register Webhook

```bash
curl -X POST http://localhost:3000/api/v1/webhooks \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://your-domain.com/webhooks"
  }'

# Save the webhook secret from the response for signature verification
```

## Webhook Integration

Webhooks are sent for all completed transactions with HMAC-SHA256 signatures for security.

### Verify Webhook Signature (Python)

```python
import hmac
import hashlib

def verify_webhook(request):
    signature = request.headers['X-Webhook-Signature']
    payload = request.body  # raw bytes
    secret = "YOUR_WEBHOOK_SECRET"

    expected = "sha256=" + hmac.new(
        secret.encode(),
        payload,
        hashlib.sha256
    ).hexdigest()

    return hmac.compare_digest(expected, signature)
```

See [API.md](API.md#webhook-delivery) for verification examples in Node.js and Rust.

## Testing

### Automated Integration Tests

```bash
# Generate API key
API_KEY=$(./scripts/create_api_key.sh)

# Run all tests
./scripts/test_api.sh $API_KEY
```

### Test Webhooks

```bash
# Use webhook.site or RequestBin for testing
./scripts/test_webhooks.sh $API_KEY
```

### Manual Testing

```bash
# Start the service
docker compose up

# In another terminal
curl http://localhost:3000/health

# Create accounts and transactions using curl examples above
```

## Future Enhancements

- Multi-currency support
- Webhook retry queue
- Rate limiting per API key
- OpenTelemetry integration
- Read replicas for scaling
- GraphQL API
