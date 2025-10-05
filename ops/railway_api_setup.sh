#!/bin/bash

# Railway API Setup Script for LogLine Universe
# Uses Railway GraphQL API to set up complete infrastructure

set -e

# Configuration
RAILWAY_TOKEN="790ec773-6667-4608-a595-3b40d689d71d"
PROJECT_NAME="logline-universe"
RAILWAY_API="https://backboard.railway.app/graphql/v2"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Setting up LogLine Universe on Railway via API${NC}"

# Function to make GraphQL requests
railway_graphql() {
    local query="$1"
    curl -s -X POST \
        -H "Authorization: Bearer $RAILWAY_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{\"query\": \"$query\"}" \
        "$RAILWAY_API"
}

# Function to extract value from JSON response
extract_value() {
    local json="$1"
    local path="$2"
    echo "$json" | python3 -c "
import sys, json
data = json.load(sys.stdin)
try:
    result = data
    for key in '$path'.split('.'):
        if key.isdigit():
            result = result[int(key)]
        else:
            result = result[key]
    print(result)
except:
    print('')
"
}

echo -e "${BLUE}📋 Step 1: Creating Railway project...${NC}"

# Create project
PROJECT_QUERY="mutation { projectCreate(input: { name: \"$PROJECT_NAME\", description: \"LogLine Universe - Distributed logging and identity system\", isPublic: false }) { id name } }"

PROJECT_RESPONSE=$(railway_graphql "$PROJECT_QUERY")
PROJECT_ID=$(extract_value "$PROJECT_RESPONSE" "data.projectCreate.id")

if [ -z "$PROJECT_ID" ]; then
    echo -e "${RED}❌ Failed to create project${NC}"
    echo "$PROJECT_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Project created: $PROJECT_ID${NC}"

echo -e "${BLUE}🗄️  Step 2: Adding PostgreSQL database...${NC}"

# Add PostgreSQL service
POSTGRES_QUERY='
mutation {
  serviceCreate(input: {
    projectId: "'$PROJECT_ID'"
    name: "postgres"
    source: {
      image: "postgres:15"
    }
  }) {
    id
    name
  }
}
'

POSTGRES_RESPONSE=$(railway_graphql "$POSTGRES_QUERY")
POSTGRES_SERVICE_ID=$(extract_value "$POSTGRES_RESPONSE" "data.serviceCreate.id")

if [ -z "$POSTGRES_SERVICE_ID" ]; then
    echo -e "${RED}❌ Failed to create PostgreSQL service${NC}"
    echo "$POSTGRES_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ PostgreSQL service created: $POSTGRES_SERVICE_ID${NC}"

echo -e "${BLUE}🔴 Step 3: Adding Redis cache...${NC}"

# Add Redis service
REDIS_QUERY='
mutation {
  serviceCreate(input: {
    projectId: "'$PROJECT_ID'"
    name: "redis"
    source: {
      image: "redis:7-alpine"
    }
  }) {
    id
    name
  }
}
'

REDIS_RESPONSE=$(railway_graphql "$REDIS_QUERY")
REDIS_SERVICE_ID=$(extract_value "$REDIS_RESPONSE" "data.serviceCreate.id")

if [ -z "$REDIS_SERVICE_ID" ]; then
    echo -e "${RED}❌ Failed to create Redis service${NC}"
    echo "$REDIS_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Redis service created: $REDIS_SERVICE_ID${NC}"

echo -e "${BLUE}⚙️  Step 4: Setting up environment variables...${NC}"

# Set environment variables for the project
ENV_VARS=(
    "LOGLINE_ENV:production"
    "LOGLINE_NODE_NAME:logline-production"
    "LOGLINE_HTTP_BIND:0.0.0.0:8080"
    "LOGLINE_WS_BIND:0.0.0.0:8081"
)

for env_var in "${ENV_VARS[@]}"; do
    IFS=':' read -r key value <<< "$env_var"
    
    ENV_QUERY='
    mutation {
      variableUpsert(input: {
        projectId: "'$PROJECT_ID'"
        name: "'$key'"
        value: "'$value'"
      }) {
        id
        name
      }
    }
    '
    
    ENV_RESPONSE=$(railway_graphql "$ENV_QUERY")
    echo -e "${YELLOW}📝 Set $key = $value${NC}"
done

echo -e "${BLUE}🐳 Step 5: Creating service deployments...${NC}"

# Create ID service
ID_QUERY='
mutation {
  serviceCreate(input: {
    projectId: "'$PROJECT_ID'"
    name: "logline-id"
    source: {
      repo: "danvoulez/UniverseLogLine"
      rootDirectory: "."
      dockerfile: "Dockerfile.id"
    }
  }) {
    id
    name
  }
}
'

ID_RESPONSE=$(railway_graphql "$ID_QUERY")
ID_SERVICE_ID=$(extract_value "$ID_RESPONSE" "data.serviceCreate.id")

echo -e "${GREEN}✅ ID service created: $ID_SERVICE_ID${NC}"

# Create timeline service
TIMELINE_QUERY='
mutation {
  serviceCreate(input: {
    projectId: "'$PROJECT_ID'"
    name: "logline-timeline"
    source: {
      repo: "danvoulez/UniverseLogLine"
      rootDirectory: "."
      dockerfile: "Dockerfile.timeline"
    }
  }) {
    id
    name
  }
}
'

TIMELINE_RESPONSE=$(railway_graphql "$TIMELINE_QUERY")
TIMELINE_SERVICE_ID=$(extract_value "$TIMELINE_RESPONSE" "data.serviceCreate.id")

echo -e "${GREEN}✅ Timeline service created: $TIMELINE_SERVICE_ID${NC}"

# Create rules service
RULES_QUERY='
mutation {
  serviceCreate(input: {
    projectId: "'$PROJECT_ID'"
    name: "logline-rules"
    source: {
      repo: "danvoulez/UniverseLogLine"
      rootDirectory: "."
      dockerfile: "Dockerfile.rules"
    }
  }) {
    id
    name
  }
}
'

RULES_RESPONSE=$(railway_graphql "$RULES_QUERY")
RULES_SERVICE_ID=$(extract_value "$RULES_RESPONSE" "data.serviceCreate.id")

echo -e "${GREEN}✅ Rules service created: $RULES_SERVICE_ID${NC}"

# Create engine service
ENGINE_QUERY='
mutation {
  serviceCreate(input: {
    projectId: "'$PROJECT_ID'"
    name: "logline-engine"
    source: {
      repo: "danvoulez/UniverseLogLine"
      rootDirectory: "."
      dockerfile: "Dockerfile.engine"
    }
  }) {
    id
    name
  }
}
'

ENGINE_RESPONSE=$(railway_graphql "$ENGINE_QUERY")
ENGINE_SERVICE_ID=$(extract_value "$ENGINE_RESPONSE" "data.serviceCreate.id")

echo -e "${GREEN}✅ Engine service created: $ENGINE_SERVICE_ID${NC}"

echo -e "${BLUE}🔗 Step 6: Getting connection URLs...${NC}"

# Get project details including service URLs
PROJECT_DETAILS_QUERY='
query {
  project(id: "'$PROJECT_ID'") {
    id
    name
    services {
      edges {
        node {
          id
          name
          variables {
            edges {
              node {
                name
                value
              }
            }
          }
        }
      }
    }
  }
}
'

PROJECT_DETAILS=$(railway_graphql "$PROJECT_DETAILS_QUERY")

echo -e "${GREEN}🎉 Railway setup complete!${NC}"

echo -e "${BLUE}📊 Project Summary:${NC}"
echo "• Project ID: $PROJECT_ID"
echo "• PostgreSQL Service: $POSTGRES_SERVICE_ID"
echo "• Redis Service: $REDIS_SERVICE_ID"
echo "• ID Service: $ID_SERVICE_ID"
echo "• Timeline Service: $TIMELINE_SERVICE_ID"
echo "• Rules Service: $RULES_SERVICE_ID"
echo "• Engine Service: $ENGINE_SERVICE_ID"

echo -e "${YELLOW}📝 Next Steps:${NC}"
echo "1. Wait for services to deploy (5-10 minutes)"
echo "2. Run database migrations"
echo "3. Test service endpoints"
echo "4. Set up monitoring"

echo -e "${BLUE}🔗 Railway Dashboard:${NC}"
echo "https://railway.app/project/$PROJECT_ID"

# Save project info
cat > railway_project_info.json << EOF
{
  "project_id": "$PROJECT_ID",
  "services": {
    "postgres": "$POSTGRES_SERVICE_ID",
    "redis": "$REDIS_SERVICE_ID",
    "id": "$ID_SERVICE_ID",
    "timeline": "$TIMELINE_SERVICE_ID",
    "rules": "$RULES_SERVICE_ID",
    "engine": "$ENGINE_SERVICE_ID"
  },
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo -e "${GREEN}💾 Project info saved to railway_project_info.json${NC}"
