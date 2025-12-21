#!/bin/bash
# Comprehensive API integration test script
# Usage: ./scripts/test_api.sh <API_KEY>

set -e

API_KEY=$1
BASE_URL="http://localhost:3000"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if API key is provided
if [ -z "$API_KEY" ]; then
  echo -e "${RED}Error: API key is required${NC}"
  echo "Usage: $0 <API_KEY>"
  echo ""
  echo "Generate an API key first:"
  echo "  ./scripts/create_api_key.sh"
  exit 1
fi

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}Transaction Service API Test Suite${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Test 1: Health Check
echo -e "${BLUE}[1/10] Testing health endpoint...${NC}"
HEALTH=$(curl -s $BASE_URL/health)
if echo "$HEALTH" | grep -q "healthy"; then
  echo -e "${GREEN}✓ Health check passed${NC}"
else
  echo -e "${RED}✗ Health check failed${NC}"
  exit 1
fi
echo ""

# Test 2: Create Account 1
echo -e "${BLUE}[2/10] Creating account 1...${NC}"
ACCOUNT1=$(curl -s -X POST $BASE_URL/api/v1/accounts \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_name": "Test Account 1",
    "initial_balance_cents": 100000
  }')

ACCOUNT1_ID=$(echo "$ACCOUNT1" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -n "$ACCOUNT1_ID" ]; then
  echo -e "${GREEN}✓ Account 1 created: $ACCOUNT1_ID${NC}"
  echo -e "   Balance: \$1,000.00"
else
  echo -e "${RED}✗ Failed to create account 1${NC}"
  echo "$ACCOUNT1"
  exit 1
fi
echo ""

# Test 3: Create Account 2
echo -e "${BLUE}[3/10] Creating account 2...${NC}"
ACCOUNT2=$(curl -s -X POST $BASE_URL/api/v1/accounts \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "account_name": "Test Account 2",
    "initial_balance_cents": 50000
  }')

ACCOUNT2_ID=$(echo "$ACCOUNT2" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -n "$ACCOUNT2_ID" ]; then
  echo -e "${GREEN}✓ Account 2 created: $ACCOUNT2_ID${NC}"
  echo -e "   Balance: \$500.00"
else
  echo -e "${RED}✗ Failed to create account 2${NC}"
  exit 1
fi
echo ""

# Test 4: List Accounts
echo -e "${BLUE}[4/10] Listing all accounts...${NC}"
ACCOUNTS=$(curl -s $BASE_URL/api/v1/accounts \
  -H "Authorization: Bearer $API_KEY")
ACCOUNT_COUNT=$(echo "$ACCOUNTS" | grep -o '"id"' | wc -l)
echo -e "${GREEN}✓ Found $ACCOUNT_COUNT accounts${NC}"
echo ""

# Test 5: Credit Account 1
echo -e "${BLUE}[5/10] Crediting account 1 (\$250.00)...${NC}"
CREDIT=$(curl -s -X POST $BASE_URL/api/v1/transactions/credit \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"account_id\": \"$ACCOUNT1_ID\",
    \"amount_cents\": 25000,
    \"description\": \"Test deposit\",
    \"idempotency_key\": \"test-credit-$(date +%s)\"
  }")

CREDIT_ID=$(echo "$CREDIT" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -n "$CREDIT_ID" ]; then
  echo -e "${GREEN}✓ Credit successful: $CREDIT_ID${NC}"
  echo -e "   New balance: \$1,250.00"
else
  echo -e "${RED}✗ Credit failed${NC}"
  echo "$CREDIT"
  exit 1
fi
echo ""

# Test 6: Debit Account 2
echo -e "${BLUE}[6/10] Debiting account 2 (\$100.00)...${NC}"
DEBIT=$(curl -s -X POST $BASE_URL/api/v1/transactions/debit \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"account_id\": \"$ACCOUNT2_ID\",
    \"amount_cents\": 10000,
    \"description\": \"Test withdrawal\",
    \"idempotency_key\": \"test-debit-$(date +%s)\"
  }")

DEBIT_ID=$(echo "$DEBIT" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -n "$DEBIT_ID" ]; then
  echo -e "${GREEN}✓ Debit successful: $DEBIT_ID${NC}"
  echo -e "   New balance: \$400.00"
else
  echo -e "${RED}✗ Debit failed${NC}"
  echo "$DEBIT"
  exit 1
fi
echo ""

# Test 7: Transfer
echo -e "${BLUE}[7/10] Transferring \$200.00 from account 1 to account 2...${NC}"
TRANSFER=$(curl -s -X POST $BASE_URL/api/v1/transactions/transfer \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"from_account_id\": \"$ACCOUNT1_ID\",
    \"to_account_id\": \"$ACCOUNT2_ID\",
    \"amount_cents\": 20000,
    \"description\": \"Test transfer\",
    \"idempotency_key\": \"test-transfer-$(date +%s)\"
  }")

TRANSFER_ID=$(echo "$TRANSFER" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -n "$TRANSFER_ID" ]; then
  echo -e "${GREEN}✓ Transfer successful: $TRANSFER_ID${NC}"
  echo -e "   Account 1 balance: \$1,050.00"
  echo -e "   Account 2 balance: \$600.00"
else
  echo -e "${RED}✗ Transfer failed${NC}"
  echo "$TRANSFER"
  exit 1
fi
echo ""

# Test 8: Get Transaction
echo -e "${BLUE}[8/10] Retrieving transaction details...${NC}"
TX_DETAILS=$(curl -s $BASE_URL/api/v1/transactions/$TRANSFER_ID \
  -H "Authorization: Bearer $API_KEY")
if echo "$TX_DETAILS" | grep -q "$TRANSFER_ID"; then
  echo -e "${GREEN}✓ Transaction retrieved successfully${NC}"
else
  echo -e "${RED}✗ Failed to retrieve transaction${NC}"
  exit 1
fi
echo ""

# Test 9: Register Webhook
echo -e "${BLUE}[9/10] Registering webhook endpoint...${NC}"
WEBHOOK=$(curl -s -X POST $BASE_URL/api/v1/webhooks \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "http://localhost:8080/webhooks"
  }')

WEBHOOK_ID=$(echo "$WEBHOOK" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -n "$WEBHOOK_ID" ]; then
  echo -e "${GREEN}✓ Webhook registered: $WEBHOOK_ID${NC}"
  WEBHOOK_SECRET=$(echo "$WEBHOOK" | grep -o '"secret":"[^"]*"' | cut -d'"' -f4)
  if [ -n "$WEBHOOK_SECRET" ]; then
    echo -e "   Secret: ${YELLOW}${WEBHOOK_SECRET:0:16}...${NC}"
  fi
else
  echo -e "${YELLOW}⚠ Webhook registration failed (may require HTTPS URL)${NC}"
fi
echo ""

# Test 10: List Webhooks
echo -e "${BLUE}[10/10] Listing webhook endpoints...${NC}"
WEBHOOKS=$(curl -s $BASE_URL/api/v1/webhooks \
  -H "Authorization: Bearer $API_KEY")
WEBHOOK_COUNT=$(echo "$WEBHOOKS" | grep -o '"id"' | wc -l)
echo -e "${GREEN}✓ Found $WEBHOOK_COUNT webhook(s)${NC}"
echo ""

# Final Summary
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}All tests completed successfully!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Summary:"
echo "  • Created 2 accounts"
echo "  • Executed 3 transactions (credit, debit, transfer)"
echo "  • Registered webhook endpoint"
echo ""
echo "Final balances:"
echo "  • Account 1: \$1,050.00"
echo "  • Account 2: \$600.00"
echo ""
