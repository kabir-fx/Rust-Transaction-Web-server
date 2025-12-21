# Transaction Service - Design Specification

## Table of Contents

1. [System Architecture](#system-architecture)
2. [API Design](#api-design)
3. [Database Schema](#database-schema)
4. [Webhook Design](#webhook-design)
5. [Security Considerations](#security-considerations)
6. [Operational Considerations](#operational-considerations)
7. [Trade-offs and Assumptions](#trade-offs-and-assumptions)

---

## System Architecture

![arch_diagram](/assets/arch_diagram.svg)

### Component Overview

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Client    │────────▶│  Web Server  │────────▶│  PostgreSQL │
│  (HTTP/S)   │◀────────│   (Axum)     │◀────────│  Database   │
└─────────────┘         └──────────────┘         └─────────────┘
                              │
                              │ (async HTTP)
                              ▼
                     ┌──────────────────┐
                     │ Webhook Endpoints│
                     │  (External URLs) │
                     └──────────────────┘
```

### Technology Stack

| Layer               | Technology                         | Rationale                                                        |
| ------------------- | ---------------------------------- | ---------------------------------------------------------------- |
| **Web Framework**   | Axum 0.8                           | Modern async framework, tower ecosystem, type-safe extractors    |
| **Runtime**         | Tokio                              | Industry-standard async runtime, excellent performance           |
| **Database**        | PostgreSQL 16                      | ACID transactions, rich data types (JSONB), proven reliability   |
| **Database Driver** | sqlx 0.8                           | Compile-time query verification, async-native, migration support |
| **Authentication**  | Custom API keys                    | Simple, stateless, suitable for service-to-service auth          |
| **Serialization**   | serde_json                         | De facto standard for JSON in Rust                               |
| **Cryptography**    | SHA-256 (sha2), HMAC-SHA256 (hmac) | Industry-standard hashing and signing                            |
| **Logging**         | tracing                            | Structured logging with context propagation                      |
| **Deployment**      | Docker Compose                     | One-command local setup, easy for development and testing        |

### Request Flow

1. **HTTP Request** arrives at Axum server
2. **TraceLayer** logs request details
3. **Auth Middleware** validates API key (except `/health`)
   - Extracts `Authorization: Bearer <key>` header
   - Hashes key with SHA-256
   - Queries database for matching hash
   - Injects `AuthContext` with `api_key_id` and `business_name`
4. **Route Handler** receives request
   - Validates ownership (account belongs to authenticated business)
   - Calls service layer
5. **Service Layer** executes business logic
   - Checks idempotency key for duplicates
   - Starts database transaction
   - Updates account balances atomically
   - Records transaction
   - Sends webhook notifications synchronously
6. **Response** returned to client

---

## API Design

### RESTful Conventions

- **Versioning**: `/api/v1` prefix for all endpoints (allows future v2)
- **Resource naming**: Plural nouns (`/accounts`, `/transactions`, `/webhooks`)
- **HTTP methods**:
  - POST for creation
  - GET for retrieval
  - DELETE for deletion (soft delete)
- **Status codes**:
  - 200 OK - Successful retrieval
  - 201 Created - Successful creation
  - 204 No Content - Successful deletion
  - 400 Bad Request - Invalid input
  - 401 Unauthorized - Missing/invalid API key
  - 404 Not Found - Resource doesn't exist
  - 500 Internal Server Error - Server error

### Authentication Strategy

**Bearer Token Authentication**

```
Authorization: Bearer <api_key>
```

- API keys are generated server-side (32 random bytes, hex-encoded = 64 chars)
- Keys are hashed with SHA-256 before storage (never stored plaintext)
- Middleware validates on every request (except `/health`)
- Failed authentication returns 401 with clear error message

### Idempotency Design

**Purpose**: Prevent duplicate transactions from network retries or client errors

**Implementation**:

- Optional `idempotency_key` field in transaction requests
- Unique constraint on `transactions.idempotency_key` column
- If duplicate key detected:
  - Return existing transaction (200 OK, not 201 Created)
  - No balance changes occur
  - Same response as original request

**Recommendation**: Clients should use UUID v4 or `{business_id}-{operation_id}` format

### Error Response Format

All errors return consistent JSON structure:

```json
{
  "error": "Insufficient funds",
  "details": "Account balance: 5000 cents, requested: 10000 cents"
}
```

Implemented via centralized `AppError` enum with `IntoResponse` trait.

---

## Database Schema

### Tables Overview
| Table | Purpose | Key Relationships |
|-------|---------|------------------|
| `api_keys` | Business authentication | Parent of accounts & webhooks |
| `accounts` | Store account balances | Referenced by transactions |
| `transactions` | Financial operations | References accounts (from/to) |
| `webhook_endpoints` | Registered webhook URLs | Child of api_keys |
| `webhook_events` | Webhook delivery audit | References webhooks & transactions |

### Key Design Decisions

#### 1. Money Storage - Integer Cents

**Decision**: Store all amounts as `BIGINT` in cents (or smallest currency unit)

**Rationale**:

- Avoids floating-point precision errors (critical for financial data)
- `BIGINT` supports ±9 quadrillion cents (±90 trillion dollars)
- Simple arithmetic (addition/subtraction) without rounding concerns

**Trade-off**: Clients must convert to/from display format (e.g., `$100.00 = 10000` cents)

#### 2. Idempotency via Unique Constraint

**Decision**: Database `UNIQUE` constraint on `idempotency_key`

**Rationale**:

- Database enforces uniqueness atomically (race-condition safe)
- No application-level distributed locking required
- Simple to implement and understand

**Trade-off**: Relies on database constraint violations (slight performance cost)

#### 3. Soft Deletes for Webhooks

**Decision**: `is_active` flag instead of `DELETE`

**Rationale**:

- Preserves audit trail (`webhook_events` references remain valid)
- Allows "undelete" functionality
- Historical reporting still works

**Trade-off**: Requires filtering inactive endpoints in queries

#### 4. JSONB Metadata

**Decision**: `metadata JSONB` column for extensibility

**Rationale**:

- Future-proof for additional data without schema changes
- Indexable and queryable (unlike TEXT)
- Supports nested structures

**Current usage**: Not actively used but available for client-specific data

### Index Strategy

| Table               | Index                              | Purpose                                        |
| ------------------- | ---------------------------------- | ---------------------------------------------- |
| `api_keys`          | `key_hash`                         | Fast authentication lookup (most common query) |
| `accounts`          | `api_key_id`                       | List accounts for a business                   |
| `transactions`      | `from_account_id, created_at DESC` | Account transaction history                    |
| `transactions`      | `to_account_id, created_at DESC`   | Incoming transactions                          |
| `transactions`      | `idempotency_key`                  | Duplicate detection                            |
| `webhook_endpoints` | `api_key_id`                       | List webhooks for business                     |
| `webhook_events`    | `transaction_id`                   | Webhook delivery audit trail                   |

---

## Webhook Design

### Security Model

**HMAC-SHA256 Signature**: Every webhook includes cryptographic signature

**Process**:

1. Generate random 32-byte secret (64 hex chars) when endpoint registered
2. Send secret to user ONCE (in registration response)
3. For each webhook:
   - Serialize payload to JSON string
   - Compute `HMAC-SHA256(secret, payload)`
   - Send as `X-Webhook-Signature: sha256=<hex>` header

**Client Verification**:

```python
import hmac
import hashlib

def verify_signature(payload_bytes, signature_header, secret):
    expected = "sha256=" + hmac.new(
        secret.encode(),
        payload_bytes,
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(expected, signature_header)
```

### Delivery Guarantees

**Current**: Best-effort, at-most-once delivery

- Webhook sent synchronously after transaction completes
- Individual webhook failures are logged but don't fail the transaction
- 5-second timeout per webhook
- No automatic retries (future enhancement)

**Reliability Recommendations**:

- Clients should accept duplicate webhooks (idempotent processing)
- Clients should return 200 OK quickly (offload processing to queue)
- Failed deliveries are logged in `webhook_events` table for manual retry

### Event Payload Structure

```json
{
  "event_id": "uuid-v4",
  "event_type": "transaction.completed",
  "timestamp": "2025-12-21T19:00:00Z",
  "data": {
    "transaction": {
      "id": "uuid",
      "transaction_type": "transfer",
      "from_account_id": "uuid",
      "to_account_id": "uuid",
      "amount_cents": 25000,
      "currency": "USD",
      "description": "Payment",
      "status": "completed",
      "created_at": "2025-12-21T19:00:00Z"
    }
  }
}
```

### HTTP Headers Sent

```
Content-Type: application/json
X-Webhook-Signature: sha256=<hex_signature>
X-Webhook-Event-Id: <uuid>
```

---

## Security Considerations

### API Key Security

**Generation**: Cryptographically secure random bytes (`rand::thread_rng`)

**Storage**: SHA-256 hash only (one-way function, cannot reverse)

**Transmission**: HTTPS required in production (Docker Compose uses HTTP for localhost only)

### SQL Injection Prevention

**All database queries use parameterized statements via sqlx**:

```rust
// ✅ Safe - parameterized
sqlx::query("SELECT * FROM accounts WHERE id = $1")
    .bind(account_id)

// ❌ Never done - string interpolation
sqlx::query(&format!("SELECT * FROM accounts WHERE id = {}", id))
```

### Balance Validation

**Database constraints**:

```sql
CONSTRAINT positive_balance CHECK (balance_cents >= 0)
CONSTRAINT positive_amount CHECK (amount_cents > 0)
```

**Application logic**: Checks sufficient funds before debit/transfer

### HTTPS for Webhooks

- Production webhooks MUST use HTTPS
- HTTP allowed ONLY for localhost/127.0.0.1 (development)
- Enforced in `validate_webhook_url()` function

---

## Operational Considerations

### Logging Strategy

**Structured logging with `tracing`**:

- **Info**: Normal operations (server start, transaction completion)
- **Warn**: Recoverable errors (webhook delivery failure)
- **Error**: Critical failures (database connection lost)

**Context propagation**: Request ID tracked across service calls

**Configuration**: `RUST_LOG` environment variable (default: `info`)

### Database Connection Pooling

**sqlx connection pool**:

- Minimum idle connections: 5
- Maximum connections: 20
- Idle timeout: 10 minutes
- Connection health checks

### Docker Deployment

**Multi-stage build**:

1. Builder stage: Compile Rust (release mode)
2. Runtime stage: Minimal Debian image with binary only
3. Reduces image size (~100MB vs ~2GB)

**Health checks**:

- Database: `pg_isready` command
- Application: `GET /health` endpoint (future: Kubernetes probes)

### Observability

**Current**: Structured logs to stdout

**Future enhancements**:

- OpenTelemetry integration (tracing/metrics)
- Prometheus metrics endpoint
- Grafana dashboards
- Error tracking (Sentry)

---

## Trade-offs and Assumptions

### Assumptions

1. **Single Currency**: USD only for MVP
   - Multi-currency requires exchange rates, more complex reporting
2. **No Transaction Reversal**: Completed transactions are final
   - Refunds require new credit transaction
3. **No Account Deletion**: Accounts are permanent
   - Can add `is_active` flag if needed
4. **Service-to-Service Auth**: API keys designed for backend services
   - Not suitable for browser-based clients (exposes key)
5. **Single Region**: No geographic distribution
   - Database is single-instance PostgreSQL

### Trade-offs

| Decision                             | Benefit                          | Cost                       | Mitigation                                                      |
| ------------------------------------ | -------------------------------- | -------------------------- | --------------------------------------------------------------- |
| **No retry logic for webhooks**      | Simpler implementation           | Lower reliability          | Log failures for manual retry; document idempotency requirement |
| **Synchronous webhook sending**      | Simpler code, immediate feedback | Slows transaction response | Use 5s timeout; future: async queue                             |
| **API keys in Authorization header** | Standard HTTP practice           | Client must secure keys    | Document best practices; future: OAuth2                         |
| **Balance stored in database**       | ACID guarantees                  | No caching (performance)   | Indexed queries; future: read replicas                          |
| **Single database instance**         | Simple deployment                | Single point of failure    | Docker volumes for persistence; future: replicas                |
| **No rate limiting**                 | Simpler MVP                      | Vulnerable to abuse        | Add tower-governor middleware in future                         |

---
