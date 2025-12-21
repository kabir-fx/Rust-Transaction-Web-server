#!/bin/bash
# Script to generate and register a new API key
# Usage: ./scripts/create_api_key.sh [business_name]

set -e

BUSINESS_NAME=${1:-"Test Business"}

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Generating API key for: $BUSINESS_NAME${NC}"

# Generate random API key (32 bytes = 64 hex chars)
API_KEY=$(openssl rand -hex 32)

# Hash the API key with SHA-256
KEY_HASH=$(echo -n "$API_KEY" | shasum -a 256 | awk '{print $1}')

# Insert into database
echo -e "${BLUE}Storing API key in database...${NC}"

docker compose exec -T db psql -U postgres -d transactions -c \
  "INSERT INTO api_keys (key_hash, business_name) VALUES ('$KEY_HASH', '$BUSINESS_NAME')" \
  > /dev/null

echo -e "${GREEN}✓ API key created successfully!${NC}"
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}API Key (save this - shown only once):${NC}"
echo ""
echo "$API_KEY"
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Use this in the Authorization header:"
echo ""
echo "  Authorization: Bearer $API_KEY"
echo ""
echo "Example:"
echo ""
echo "  export API_KEY=\"$API_KEY\""
echo "  curl -H \"Authorization: Bearer \$API_KEY\" http://localhost:3000/api/v1/accounts"
echo ""
