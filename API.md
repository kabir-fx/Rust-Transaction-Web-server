# API Documentation

Complete reference for the Transaction Service REST API.

## Base URL

```
http://localhost:3000
```

---

## Authentication

All API endpoints (except `/health`) require authentication via API key.

### Header Format

```http
Authorization: Bearer <your_api_key>
```

### Generating an API Key

Use the provided script to generate an API key:

```bash
./scripts/create_api_key.sh
```

**Important**: The API key is shown only once. Store it securely.

### Authentication Errors

**401 Unauthorized**

```json
{
  "error": "Invalid or missing API key"
}
```

---

## Endpoints

- [Health Check](#health-check)
- [Accounts](#accounts)
  - [Create Account](#create-account)
  - [List Accounts](#list-accounts)
  - [Get Account](#get-account)
- [Transactions](#transactions)
  - [Credit Transaction](#credit-transaction)
  - [Debit Transaction](#debit-transaction)
  - [Transfer Transaction](#transfer-transaction)
  - [Get Transaction](#get-transaction)
- [Webhooks](#webhooks)
  - [Register Webhook](#register-webhook)
  - [List Webhooks](#list-webhooks)
  - [Delete Webhook](#delete-webhook)

---

## Health Check

Check service health and database connectivity.

**Endpoint**: `GET /health`

**Authentication**: None required

### Example Request

```bash
curl http://localhost:3000/health
```

### Response (200 OK)

```json
{
  "status": "healthy",
  "database": "connected",
  "timestamp": "2025-12-21T19:30:00Z"
}
```

---

## Accounts

### Create Account

Create a new account for the authenticated business.

**Endpoint**: `POST /api/v1/accounts`

**Authentication**: Required

#### Request Body

```json
{
  "account_name": "Primary Checking",
  "initial_balance_cents": 100000
}
```

| Field                   | Type    | Required | Description                            |
| ----------------------- | ------- | -------- | -------------------------------------- |
| `account_name`          | string  | Yes      | Human-readable account name            |
| `initial_balance_cents` | integer | No       | Starting balance in cents (default: 0) |

#### Example Request

```bash
curl -X POST http://localhost:3000/api/v1/accounts \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_name": "Primary Checking",
    "initial_balance_cents": 100000
  }'
```

### Response (201 Created)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "account_name": "Primary Checking",
  "balance_cents": 100000,
  "currency": "USD",
  "created_at": "2025-12-21T19:00:00Z"
}
```

---

### List Accounts

Retrieve all accounts for the authenticated business.

**Endpoint**: `GET /api/v1/accounts`

**Authentication**: Required

#### Example Request

```bash
curl http://localhost:3000/api/v1/accounts \
  -H "Authorization: Bearer YOUR_API_KEY"
```

#### Response (200 OK)

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "account_name": "Primary Checking",
    "balance_cents": 100000,
    "currency": "USD",
    "created_at": "2025-12-21T19:00:00Z"
  },
  {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "account_name": "Savings",
    "balance_cents": 500000,
    "currency": "USD",
    "created_at": "2025-12-21T19:05:00Z"
  }
]
```

---

### Get Account

Retrieve a specific account by ID.

**Endpoint**: `GET /api/v1/accounts/{id}`

**Authentication**: Required

#### Path Parameters

| Parameter | Type | Description |
| --------- | ---- | ----------- |
| `id`      | UUID | Account ID  |

#### Example Request

```bash
curl http://localhost:3000/api/v1/accounts/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer YOUR_API_KEY"
```

#### Response (200 OK)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "account_name": "Primary Checking",
  "balance_cents": 100000,
  "currency": "USD",
  "created_at": "2025-12-21T19:00:00Z"
}
```

#### Error Responses

**404 Not Found**

```json
{
  "error": "Account not found"
}
```

---

## Transactions

### Credit Transaction

Add money to an account.

**Endpoint**: `POST /api/v1/transactions/credit`

**Authentication**: Required

#### Request Body

```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440000",
  "amount_cents": 50000,
  "description": "Initial deposit",
  "idempotency_key": "deposit-2025-001"
}
```

| Field             | Type    | Required | Description                          |
| ----------------- | ------- | -------- | ------------------------------------ |
| `account_id`      | UUID    | Yes      | Account to credit                    |
| `amount_cents`    | integer | Yes      | Amount to add in cents (must be > 0) |
| `description`     | string  | No       | Transaction description              |
| `idempotency_key` | string  | No       | Unique key to prevent duplicates     |

#### Example Request

```bash
curl -X POST http://localhost:3000/api/v1/transactions/credit \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "550e8400-e29b-41d4-a716-446655440000",
    "amount_cents": 50000,
    "description": "Initial deposit",
    "idempotency_key": "deposit-2025-001"
  }'
```

#### Response (201 Created)

```json
{
  "id": "770e8400-e29b-41d4-a716-446655440002",
  "transaction_type": "credit",
  "from_account_id": null,
  "to_account_id": "550e8400-e29b-41d4-a716-446655440000",
  "amount_cents": 50000,
  "currency": "USD",
  "description": "Initial deposit",
  "status": "completed",
  "created_at": "2025-12-21T19:30:00Z"
}
```

---

### Debit Transaction

Remove money from an account.

**Endpoint**: `POST /api/v1/transactions/debit`

**Authentication**: Required

#### Request Body

```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440000",
  "amount_cents": 10000,
  "description": "Monthly fee",
  "idempotency_key": "fee-2025-12"
}
```

| Field             | Type    | Required | Description                             |
| ----------------- | ------- | -------- | --------------------------------------- |
| `account_id`      | UUID    | Yes      | Account to debit                        |
| `amount_cents`    | integer | Yes      | Amount to remove in cents (must be > 0) |
| `description`     | string  | No       | Transaction description                 |
| `idempotency_key` | string  | No       | Unique key to prevent duplicates        |

#### Example Request

```bash
curl -X POST http://localhost:3000/api/v1/transactions/debit \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "550e8400-e29b-41d4-a716-446655440000",
    "amount_cents": 10000,
    "description": "Monthly fee"
  }'
```

#### Response (201 Created)

```json
{
  "id": "880e8400-e29b-41d4-a716-446655440003",
  "transaction_type": "debit",
  "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
  "to_account_id": null,
  "amount_cents": 10000,
  "currency": "USD",
  "description": "Monthly fee",
  "status": "completed",
  "created_at": "2025-12-21T19:35:00Z"
}
```

#### Error Responses

**400 Bad Request - Insufficient Funds**

```json
{
  "error": "Insufficient funds"
}
```

---

### Transfer Transaction

Transfer money between two accounts atomically.

**Endpoint**: `POST /api/v1/transactions/transfer`

**Authentication**: Required

#### Request Body

```json
{
  "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
  "to_account_id": "660e8400-e29b-41d4-a716-446655440001",
  "amount_cents": 25000,
  "description": "Payment for services",
  "idempotency_key": "invoice-789"
}
```

| Field             | Type    | Required | Description                               |
| ----------------- | ------- | -------- | ----------------------------------------- |
| `from_account_id` | UUID    | Yes      | Source account                            |
| `to_account_id`   | UUID    | Yes      | Destination account                       |
| `amount_cents`    | integer | Yes      | Amount to transfer in cents (must be > 0) |
| `description`     | string  | No       | Transaction description                   |
| `idempotency_key` | string  | No       | Unique key to prevent duplicates          |

#### Example Request

```bash
curl -X POST http://localhost:3000/api/v1/transactions/transfer \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
    "to_account_id": "660e8400-e29b-41d4-a716-446655440001",
    "amount_cents": 25000,
    "description": "Payment for services"
  }'
```

#### Response (201 Created)

```json
{
  "id": "990e8400-e29b-41d4-a716-446655440004",
  "transaction_type": "transfer",
  "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
  "to_account_id": "660e8400-e29b-41d4-a716-446655440001",
  "amount_cents": 25000,
  "currency": "USD",
  "description": "Payment for services",
  "status": "completed",
  "created_at": "2025-12-21T19:40:00Z"
}
```

#### Error Responses

**400 Bad Request - Insufficient Funds**

```json
{
  "error": "Insufficient funds"
}
```

**400 Bad Request - Same Account**

```json
{
  "error": "Cannot transfer to the same account"
}
```

---

### Get Transaction

Retrieve transaction details by ID.

**Endpoint**: `GET /api/v1/transactions/{id}`

**Authentication**: Required

#### Path Parameters

| Parameter | Type | Description    |
| --------- | ---- | -------------- |
| `id`      | UUID | Transaction ID |

#### Example Request

```bash
curl http://localhost:3000/api/v1/transactions/770e8400-e29b-41d4-a716-446655440002 \
  -H "Authorization: Bearer YOUR_API_KEY"
```

#### Response (200 OK)

```json
{
  "id": "770e8400-e29b-41d4-a716-446655440002",
  "transaction_type": "credit",
  "from_account_id": null,
  "to_account_id": "550e8400-e29b-41d4-a716-446655440000",
  "amount_cents": 50000,
  "currency": "USD",
  "description": "Initial deposit",
  "status": "completed",
  "created_at": "2025-12-21T19:30:00Z"
}
```

---

## Webhooks

### Register Webhook

Register a URL to receive transaction notifications.

**Endpoint**: `POST /api/v1/webhooks`

**Authentication**: Required

#### Request Body

```json
{
  "url": "https://your-domain.com/webhooks/transactions"
}
```

| Field | Type   | Required | Description                                                    |
| ----- | ------ | -------- | -------------------------------------------------------------- |
| `url` | string | Yes      | HTTPS URL to receive webhooks (HTTP localhost allowed for dev) |

#### Example Request

```bash
curl -X POST http://localhost:3000/api/v1/webhooks \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://your-domain.com/webhooks/transactions"
  }'
```

#### Response (201 Created)

```json
{
  "id": "aa0e8400-e29b-41d4-a716-446655440005",
  "url": "https://your-domain.com/webhooks/transactions",
  "secret": "a1b2c3d4e5f6...64_hex_characters",
  "is_active": true,
  "created_at": "2025-12-21T19:45:00Z"
}
```

**⚠️ Important**: The `secret` is shown only once. Store it securely for signature verification.

---

### List Webhooks

Retrieve all active webhook endpoints.

**Endpoint**: `GET /api/v1/webhooks`

**Authentication**: Required

#### Example Request

```bash
curl http://localhost:3000/api/v1/webhooks \
  -H "Authorization: Bearer YOUR_API_KEY"
```

#### Response (200 OK)

```json
[
  {
    "id": "aa0e8400-e29b-41d4-a716-446655440005",
    "url": "https://your-domain.com/webhooks/transactions",
    "secret": null,
    "is_active": true,
    "created_at": "2025-12-21T19:45:00Z"
  }
]
```

**Note**: `secret` is never returned in list/get endpoints (only during creation).

---

### Delete Webhook

Delete a webhook endpoint (soft delete).

**Endpoint**: `DELETE /api/v1/webhooks/{id}`

**Authentication**: Required

#### Path Parameters

| Parameter | Type | Description         |
| --------- | ---- | ------------------- |
| `id`      | UUID | Webhook endpoint ID |

#### Example Request

```bash
curl -X DELETE http://localhost:3000/api/v1/webhooks/aa0e8400-e29b-41d4-a716-446655440005 \
  -H "Authorization: Bearer YOUR_API_KEY"
```

#### Response (204 No Content)

No response body.

---

## Webhook Delivery

When a transaction completes, webhooks are sent to all registered endpoints.

### Webhook Payload

```json
{
  "event_id": "bb0e8400-e29b-41d4-a716-446655440006",
  "event_type": "transaction.completed",
  "timestamp": "2025-12-21T19:50:00Z",
  "data": {
    "transaction": {
      "id": "770e8400-e29b-41d4-a716-446655440002",
      "transaction_type": "transfer",
      "from_account_id": "550e8400-e29b-41d4-a716-446655440000",
      "to_account_id": "660e8400-e29b-41d4-a716-446655440001",
      "amount_cents": 25000,
      "currency": "USD",
      "description": "Payment",
      "status": "completed",
      "created_at": "2025-12-21T19:50:00Z"
    }
  }
}
```

### Webhook Headers

```http
Content-Type: application/json
X-Webhook-Signature: sha256=<hmac_sha256_hex>
X-Webhook-Event-Id: <event_uuid>
```

### Signature Verification

Verify the HMAC signature to ensure webhooks are authentic.

#### Python Example

```python
import hmac
import hashlib

def verify_webhook(request):
    signature = request.headers.get('X-Webhook-Signature')
    payload = request.body  # raw bytes
    secret = "YOUR_WEBHOOK_SECRET"  # from registration

    # Compute expected signature
    expected = "sha256=" + hmac.new(
        secret.encode(),
        payload,
        hashlib.sha256
    ).hexdigest()

    # Constant-time comparison
    return hmac.compare_digest(expected, signature)
```

#### Node.js Example

```javascript
const crypto = require("crypto");

function verifyWebhook(req) {
  const signature = req.headers["x-webhook-signature"];
  const payload = JSON.stringify(req.body);
  const secret = "YOUR_WEBHOOK_SECRET";

  const expected =
    "sha256=" +
    crypto.createHmac("sha256", secret).update(payload).digest("hex");

  return crypto.timingSafeEqual(Buffer.from(expected), Buffer.from(signature));
}
```

#### Rust Example

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

fn verify_webhook(payload: &str, signature: &str, secret: &str) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let expected = format!("sha256={}", hex::encode(result.into_bytes()));
    expected == signature
}
```

### Best Practices

1. **Verify Signatures**: Always validate `X-Webhook-Signature` before processing
2. **Idempotency**: Handle duplicate webhooks gracefully (same `event_id`)
3. **Quick Response**: Return 200 OK quickly, process asynchronously
4. **Retry Logic**: Implement your own retry/backoff for failed processing
5. **Security**: HTTPS only for production webhook URLs

---

## Error Handling

All errors return JSON with an `error` field and optional `details`.

### Common Error Codes

| Status Code | Error                   | Description                                     |
| ----------- | ----------------------- | ----------------------------------------------- |
| 400         | `Bad Request`           | Invalid input, missing required fields          |
| 401         | `Unauthorized`          | Invalid or missing API key                      |
| 404         | `Not Found`             | Resource doesn't exist or doesn't belong to you |
| 500         | `Internal Server Error` | Unexpected server error                         |

### Example Error Response

```json
{
  "error": "Insufficient funds",
  "details": "Account balance: 5000 cents, requested: 10000 cents"
}
```

---

## Idempotency

Use `idempotency_key` to safely retry requests without creating duplicates.

### Example

First request:

```bash
curl -X POST http://localhost:3000/api/v1/transactions/credit \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "550e8400-e29b-41d4-a716-446655440000",
    "amount_cents": 1000,
    "idempotency_key": "payment-123"
  }'
# Returns 201 Created with transaction ID "abc..."
```

Retry (same key):

```bash
# Same request
# Returns 200 OK with SAME transaction ID "abc..."
# Balance is NOT changed again
```

**Recommendation**: Use UUID v4 or `{business}-{operation}` format for keys.

---
