#!/bin/bash

# Simplified Railway Deployment Script
# Uses curl with proper JSON formatting

set -e

RAILWAY_TOKEN="790ec773-6667-4608-a595-3b40d689d71d"
RAILWAY_API="https://backboard.railway.app/graphql/v2"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 Deploying LogLine Universe to Railway${NC}"

# Function to make GraphQL requests with proper JSON
railway_request() {
    local query="$1"
    curl -s -X POST \
        -H "Authorization: Bearer $RAILWAY_TOKEN" \
        -H "Content-Type: application/json" \
        -d "$query" \
        "$RAILWAY_API"
}

echo -e "${BLUE}📋 Step 1: Creating project...${NC}"

# Create project with proper JSON
PROJECT_JSON='{
  "query": "mutation { projectCreate(input: { name: \"logline-universe\", description: \"LogLine Universe - Distributed logging and identity system\", isPublic: false }) { id name } }"
}'

PROJECT_RESPONSE=$(railway_request "$PROJECT_JSON")
echo "Project response: $PROJECT_RESPONSE"

# Extract project ID using a simpler method
PROJECT_ID=$(echo "$PROJECT_RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)

if [ -z "$PROJECT_ID" ]; then
    echo -e "${RED}❌ Failed to create project${NC}"
    echo "$PROJECT_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Project created: $PROJECT_ID${NC}"

echo -e "${BLUE}🗄️ Step 2: Adding PostgreSQL...${NC}"

POSTGRES_JSON='{
  "query": "mutation { serviceCreate(input: { projectId: \"'$PROJECT_ID'\", name: \"postgres\", source: { image: \"postgres:15\" } }) { id name } }"
}'

POSTGRES_RESPONSE=$(railway_request "$POSTGRES_JSON")
POSTGRES_ID=$(echo "$POSTGRES_RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)

echo -e "${GREEN}✅ PostgreSQL created: $POSTGRES_ID${NC}"

echo -e "${BLUE}🔴 Step 3: Adding Redis...${NC}"

REDIS_JSON='{
  "query": "mutation { serviceCreate(input: { projectId: \"'$PROJECT_ID'\", name: \"redis\", source: { image: \"redis:7-alpine\" } }) { id name } }"
}'

REDIS_RESPONSE=$(railway_request "$REDIS_JSON")
REDIS_ID=$(echo "$REDIS_RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)

echo -e "${GREEN}✅ Redis created: $REDIS_ID${NC}"

echo -e "${BLUE}🐳 Step 4: Deploying services...${NC}"

# Deploy ID service
ID_JSON='{
  "query": "mutation { serviceCreate(input: { projectId: \"'$PROJECT_ID'\", name: \"logline-id\", source: { repo: \"danvoulez/UniverseLogLine\", rootDirectory: \".\", dockerfile: \"Dockerfile.id\" } }) { id name } }"
}'

ID_RESPONSE=$(railway_request "$ID_JSON")
ID_SERVICE_ID=$(echo "$ID_RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
echo -e "${GREEN}✅ ID service: $ID_SERVICE_ID${NC}"

# Deploy Timeline service
TIMELINE_JSON='{
  "query": "mutation { serviceCreate(input: { projectId: \"'$PROJECT_ID'\", name: \"logline-timeline\", source: { repo: \"danvoulez/UniverseLogLine\", rootDirectory: \".\", dockerfile: \"Dockerfile.timeline\" } }) { id name } }"
}'

TIMELINE_RESPONSE=$(railway_request "$TIMELINE_JSON")
TIMELINE_SERVICE_ID=$(echo "$TIMELINE_RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
echo -e "${GREEN}✅ Timeline service: $TIMELINE_SERVICE_ID${NC}"

# Deploy Rules service
RULES_JSON='{
  "query": "mutation { serviceCreate(input: { projectId: \"'$PROJECT_ID'\", name: \"logline-rules\", source: { repo: \"danvoulez/UniverseLogLine\", rootDirectory: \".\", dockerfile: \"Dockerfile.rules\" } }) { id name } }"
}'

RULES_RESPONSE=$(railway_request "$RULES_JSON")
RULES_SERVICE_ID=$(echo "$RULES_RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
echo -e "${GREEN}✅ Rules service: $RULES_SERVICE_ID${NC}"

# Deploy Engine service
ENGINE_JSON='{
  "query": "mutation { serviceCreate(input: { projectId: \"'$PROJECT_ID'\", name: \"logline-engine\", source: { repo: \"danvoulez/UniverseLogLine\", rootDirectory: \".\", dockerfile: \"Dockerfile.engine\" } }) { id name } }"
}'

ENGINE_RESPONSE=$(railway_request "$ENGINE_JSON")
ENGINE_SERVICE_ID=$(echo "$ENGINE_RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
echo -e "${GREEN}✅ Engine service: $ENGINE_SERVICE_ID${NC}"

echo -e "${GREEN}🎉 All services deployed!${NC}"

echo -e "${BLUE}📊 Deployment Summary:${NC}"
echo "• Project: https://railway.app/project/$PROJECT_ID"
echo "• PostgreSQL: $POSTGRES_ID"
echo "• Redis: $REDIS_ID"
echo "• ID Service: $ID_SERVICE_ID (port 8079)"
echo "• Timeline Service: $TIMELINE_SERVICE_ID (port 8080)"
echo "• Rules Service: $RULES_SERVICE_ID (port 8081)"
echo "• Engine Service: $ENGINE_SERVICE_ID (port 8082)"

# Save deployment info
cat > railway_deployment.json << EOF
{
  "project_id": "$PROJECT_ID",
  "services": {
    "postgres": "$POSTGRES_ID",
    "redis": "$REDIS_ID",
    "id": "$ID_SERVICE_ID",
    "timeline": "$TIMELINE_SERVICE_ID",
    "rules": "$RULES_SERVICE_ID",
    "engine": "$ENGINE_SERVICE_ID"
  },
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo -e "${GREEN}💾 Deployment info saved to railway_deployment.json${NC}"
echo -e "${YELLOW}⏳ Services are now building and will be available in 5-10 minutes${NC}"
