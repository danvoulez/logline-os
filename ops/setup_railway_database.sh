#!/bin/bash

# Railway Database Setup Script for LogLine Universe
# This script sets up PostgreSQL and Redis on Railway

set -e

echo "🚀 Setting up LogLine Universe Database Infrastructure on Railway"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if Railway CLI is installed
if ! command -v railway &> /dev/null; then
    echo -e "${RED}❌ Railway CLI not found. Please install it first:${NC}"
    echo "npm install -g @railway/cli"
    exit 1
fi

# Check if user is logged in to Railway
if ! railway whoami &> /dev/null; then
    echo -e "${YELLOW}⚠️  Please login to Railway first:${NC}"
    echo "railway login"
    exit 1
fi

echo -e "${BLUE}📋 Setting up Railway project...${NC}"

# Create or connect to Railway project
echo "Creating Railway project for LogLine Universe..."
railway project create logline-universe || railway link

echo -e "${BLUE}🗄️  Adding PostgreSQL database...${NC}"

# Add PostgreSQL service
railway add postgresql

echo -e "${BLUE}🔴 Adding Redis service...${NC}"

# Add Redis service  
railway add redis

echo -e "${BLUE}⚙️  Setting up environment variables...${NC}"

# Set common environment variables
railway variables set LOGLINE_ENV=production
railway variables set LOGLINE_NODE_NAME=logline-production
railway variables set LOGLINE_HTTP_BIND=0.0.0.0:8080
railway variables set LOGLINE_WS_BIND=0.0.0.0:8081

echo -e "${GREEN}✅ Railway services created successfully!${NC}"

echo -e "${YELLOW}📝 Next steps:${NC}"
echo "1. Run database migrations: ./scripts/run_migrations.sh"
echo "2. Deploy services: railway deploy"
echo "3. Check service status: railway status"

echo -e "${BLUE}🔗 Useful Railway commands:${NC}"
echo "• railway variables: View all environment variables"
echo "• railway logs: View service logs"
echo "• railway shell: Connect to service shell"
echo "• railway connect postgres: Connect to PostgreSQL"
echo "• railway connect redis: Connect to Redis"

echo -e "${GREEN}🎉 Railway database infrastructure setup complete!${NC}"
