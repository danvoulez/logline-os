# LogLine Deployment Guide for Railway

This document outlines the process for deploying the LogLine microservices architecture on Railway.

## Prerequisites

Before deploying to Railway, you'll need:

1. A Railway account with appropriate access rights
2. GitHub repositories set up for each service
3. GitHub Actions configured for CI/CD
4. Access to PostgreSQL and Redis instances
5. Domain name(s) for your services (optional)

## Infrastructure Setup

### 1. Create Railway Project

First, create a new Railway project for the LogLine system:

```bash
# Install Railway CLI if not already installed
npm i -g @railway/cli

# Login to Railway
railway login

# Create a new project
railway init logline-system
```

### 2. Set Up Shared Infrastructure

Create the shared database services that will be used by multiple microservices:

```bash
# Create PostgreSQL instance
railway add postgresql

# Create Redis instance
railway add redis

# Verify services are created
railway status
```

### 3. Set Up Environment Variables

Create a shared environment template for services:

```bash
# Create environment variables for production
railway vars set NODE_ENV=production
railway vars set RUST_LOG=info
```

## Deploying Individual Services

### 1. Foundation Services

#### logline-id

```bash
# Navigate to logline-id repository
cd logline-id

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set ID_SERVICE_PORT=8081
railway vars set DATABASE_URL=${{ POSTGRESQL_URL }}
railway vars set SECRET_KEY=your-secret-key

# Deploy the service
railway up --service logline-id

# Get the service URL
railway status
```

#### logline-timeline

```bash
# Navigate to logline-timeline repository
cd logline-timeline

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set TIMELINE_SERVICE_PORT=8082
railway vars set DATABASE_URL=${{ POSTGRESQL_URL }}
railway vars set ID_SERVICE_URL=https://logline-id-production.up.railway.app

# Deploy the service
railway up --service logline-timeline
```

### 2. Processing Services

#### logline-rules

```bash
# Navigate to logline-rules repository
cd logline-rules

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set RULES_SERVICE_PORT=8083
railway vars set REDIS_URL=${{ REDIS_URL }}

# Deploy the service
railway up --service logline-rules
```

#### logline-engine

```bash
# Navigate to logline-engine repository
cd logline-engine

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set ENGINE_SERVICE_PORT=8084
railway vars set ID_SERVICE_URL=https://logline-id-production.up.railway.app
railway vars set TIMELINE_SERVICE_URL=https://logline-timeline-production.up.railway.app
railway vars set RULES_SERVICE_URL=https://logline-rules-production.up.railway.app
railway vars set REDIS_URL=${{ REDIS_URL }}

# Deploy the service
railway up --service logline-engine
```

### 3. Network Services

#### logline-federation

```bash
# Navigate to logline-federation repository
cd logline-federation

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set FEDERATION_SERVICE_PORT=8085
railway vars set ID_SERVICE_URL=https://logline-id-production.up.railway.app
railway vars set TIMELINE_SERVICE_URL=https://logline-timeline-production.up.railway.app

# Deploy the service
railway up --service logline-federation
```

#### logline-orchestrator

```bash
# Navigate to logline-orchestrator repository
cd logline-orchestrator

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set ORCHESTRATOR_SERVICE_PORT=8086
railway vars set SERVICE_REGISTRY={"id":"https://logline-id-production.up.railway.app","timeline":"https://logline-timeline-production.up.railway.app","engine":"https://logline-engine-production.up.railway.app","rules":"https://logline-rules-production.up.railway.app","federation":"https://logline-federation-production.up.railway.app"}

# Deploy the service
railway up --service logline-orchestrator
```

### 4. Support Services

#### logline-observer

```bash
# Navigate to logline-observer repository
cd logline-observer

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set OBSERVER_SERVICE_PORT=8087
railway vars set SERVICE_REGISTRY={"id":"https://logline-id-production.up.railway.app","timeline":"https://logline-timeline-production.up.railway.app","engine":"https://logline-engine-production.up.railway.app","rules":"https://logline-rules-production.up.railway.app","federation":"https://logline-federation-production.up.railway.app","orchestrator":"https://logline-orchestrator-production.up.railway.app"}

# Deploy the service
railway up --service logline-observer
```

#### logline-onboarding

```bash
# Navigate to logline-onboarding repository
cd logline-onboarding

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set ONBOARDING_SERVICE_PORT=8088
railway vars set ID_SERVICE_URL=https://logline-id-production.up.railway.app

# Deploy the service
railway up --service logline-onboarding
```

### 5. API Gateway

```bash
# Navigate to logline-api repository
cd logline-api

# Link to Railway project
railway link

# Set service-specific environment variables
railway vars set API_SERVICE_PORT=8080
railway vars set SERVICE_REGISTRY={"id":"https://logline-id-production.up.railway.app","timeline":"https://logline-timeline-production.up.railway.app","engine":"https://logline-engine-production.up.railway.app","rules":"https://logline-rules-production.up.railway.app","federation":"https://logline-federation-production.up.railway.app","orchestrator":"https://logline-orchestrator-production.up.railway.app","observer":"https://logline-observer-production.up.railway.app","onboarding":"https://logline-onboarding-production.up.railway.app"}

# Deploy the service
railway up --service logline-api
```

## Setting Up Custom Domains

For a production deployment, you'll want to set up custom domains for your services:

```bash
# Add a custom domain to the API gateway
railway domain add api.logline.example.com --service logline-api
```

## Configuring CI/CD with GitHub Actions

Create a GitHub Actions workflow file in each repository (`.github/workflows/deploy.yml`):

```yaml
name: Deploy to Railway

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true

      - name: Install Railway CLI
        run: npm i -g @railway/cli

      - name: Deploy to Railway
        run: railway up --service [SERVICE-NAME]
        env:
          RAILWAY_TOKEN: ${{ secrets.RAILWAY_TOKEN }}
```

## Monitoring and Logging

Railway provides built-in monitoring and logging capabilities:

1. View service logs: `railway logs --service logline-api`
2. View service metrics in the Railway dashboard
3. Set up alerts for service health issues

## Scaling Services

Railway allows for easy scaling of services:

1. Navigate to the service in the Railway dashboard
2. Adjust the instance count or size as needed
3. Services will automatically scale based on the new configuration

## Backup and Disaster Recovery

1. **Database Backups**:
   - Configure automatic PostgreSQL backups in Railway
   - Set an appropriate backup retention policy

2. **Service Redundancy**:
   - Deploy critical services across multiple instances
   - Configure appropriate health checks and restart policies

3. **Disaster Recovery Plan**:
   - Document steps to recover from catastrophic failure
   - Regularly test the recovery process

## Security Considerations

1. **Environment Variables**:
   - Store all sensitive information in Railway environment variables
   - Never commit secrets to source code

2. **Network Security**:
   - Configure appropriate network policies
   - Use HTTPS for all service communication

3. **Authentication and Authorization**:
   - Implement proper authentication for all services
   - Use JWT or similar for inter-service communication

## Best Practices for Railway Deployment

1. **Use Infrastructure as Code**:
   - Define your Railway infrastructure using code
   - Version your infrastructure definitions

2. **Optimize Container Images**:
   - Use multi-stage builds for smaller images
   - Optimize Dockerfiles for faster builds

3. **Implement Health Checks**:
   - Add health check endpoints to all services
   - Configure Railway to use these endpoints

4. **Monitor Resource Usage**:
   - Regularly review resource usage
   - Optimize services that consume excessive resources

5. **Test Deployments**:
   - Use staging environments before production
   - Implement blue-green deployments for zero downtime updates

## Troubleshooting Common Issues

### Service Won't Start

1. Check the service logs: `railway logs --service logline-api`
2. Verify environment variables are set correctly
3. Check for build errors in the GitHub Actions workflow

### Service Connectivity Issues

1. Verify service URLs are correct in environment variables
2. Check network policies allow communication
3. Verify services are running and healthy

### Database Connection Issues

1. Verify DATABASE_URL is set correctly
2. Check database connectivity from the service
3. Verify database permissions

## Conclusion

This deployment guide provides a structured approach for deploying the LogLine microservices architecture on Railway. By following these steps, you can create a robust, scalable, and maintainable deployment of the LogLine system.