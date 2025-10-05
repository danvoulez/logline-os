#!/bin/bash

# Database Migration Runner for LogLine Universe
# Runs all SQL migrations in the correct order

set -e

echo "🗄️ Running LogLine Universe Database Migrations"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo -e "${RED}❌ DATABASE_URL environment variable not set${NC}"
    echo "Please set DATABASE_URL or run: railway variables"
    exit 1
fi

echo -e "${BLUE}📋 Database URL: ${DATABASE_URL}${NC}"

# Function to run a migration file
run_migration() {
    local migration_file=$1
    local migration_name=$(basename "$migration_file" .sql)
    
    echo -e "${YELLOW}🔄 Running migration: ${migration_name}${NC}"
    
    if psql "$DATABASE_URL" -f "$migration_file"; then
        echo -e "${GREEN}✅ Migration ${migration_name} completed successfully${NC}"
    else
        echo -e "${RED}❌ Migration ${migration_name} failed${NC}"
        exit 1
    fi
}

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo -e "${RED}❌ psql not found. Please install PostgreSQL client${NC}"
    echo "macOS: brew install postgresql"
    echo "Ubuntu: sudo apt-get install postgresql-client"
    exit 1
fi

echo -e "${BLUE}🔍 Testing database connection...${NC}"
if ! psql "$DATABASE_URL" -c "SELECT version();" > /dev/null; then
    echo -e "${RED}❌ Cannot connect to database${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Database connection successful${NC}"

# Create migrations tracking table
echo -e "${BLUE}📋 Setting up migration tracking...${NC}"
psql "$DATABASE_URL" -c "
CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
"

# Function to check if migration was already applied
is_migration_applied() {
    local version=$1
    local count=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM schema_migrations WHERE version = '$version';")
    [ "$count" -gt 0 ]
}

# Function to mark migration as applied
mark_migration_applied() {
    local version=$1
    psql "$DATABASE_URL" -c "INSERT INTO schema_migrations (version) VALUES ('$version');"
}

# Run migrations in order
MIGRATION_DIR="migrations"

if [ ! -d "$MIGRATION_DIR" ]; then
    echo -e "${RED}❌ Migrations directory not found: $MIGRATION_DIR${NC}"
    exit 1
fi

echo -e "${BLUE}📂 Running migrations from: $MIGRATION_DIR${NC}"

# Run main migrations
for migration_file in "$MIGRATION_DIR"/*.sql; do
    if [ -f "$migration_file" ]; then
        migration_name=$(basename "$migration_file" .sql)
        
        if is_migration_applied "$migration_name"; then
            echo -e "${YELLOW}⏭️  Skipping already applied migration: ${migration_name}${NC}"
        else
            run_migration "$migration_file"
            mark_migration_applied "$migration_name"
        fi
    fi
done

# Run timeline-specific migrations if they exist
TIMELINE_MIGRATION_DIR="timeline/migrations"
if [ -d "$TIMELINE_MIGRATION_DIR" ]; then
    echo -e "${BLUE}📂 Running timeline migrations from: $TIMELINE_MIGRATION_DIR${NC}"
    
    for migration_file in "$TIMELINE_MIGRATION_DIR"/*.sql; do
        if [ -f "$migration_file" ]; then
            migration_name="timeline_$(basename "$migration_file" .sql)"
            
            if is_migration_applied "$migration_name"; then
                echo -e "${YELLOW}⏭️  Skipping already applied migration: ${migration_name}${NC}"
            else
                run_migration "$migration_file"
                mark_migration_applied "$migration_name"
            fi
        fi
    done
fi

echo -e "${GREEN}🎉 All migrations completed successfully!${NC}"

# Show applied migrations
echo -e "${BLUE}📋 Applied migrations:${NC}"
psql "$DATABASE_URL" -c "SELECT version, applied_at FROM schema_migrations ORDER BY applied_at;"

echo -e "${BLUE}📊 Database statistics:${NC}"
psql "$DATABASE_URL" -c "
SELECT 
    schemaname,
    tablename,
    n_tup_ins as inserts,
    n_tup_upd as updates,
    n_tup_del as deletes
FROM pg_stat_user_tables 
ORDER BY schemaname, tablename;
"
