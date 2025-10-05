# LogLine Naming Conventions

This document outlines the standardized naming conventions for the LogLine microservices architecture.

## Service Names

All LogLine services follow the kebab-case naming convention:

| Service | Description |
|---------|-------------|
| `logline-core` | Shared library with common types and utilities |
| `logline-protocol` | Communication protocols and message formats |
| `logline-id` | Identity management service |
| `logline-timeline` | Timeline and span management service |
| `logline-rules` | Grammar parsing and rule execution service |
| `logline-engine` | Execution engine (formerly "motor") |
| `logline-federation` | Network communication and federation service |
| `logline-api` | Unified API gateway |
| `logline-observer` | Monitoring and analytics service |
| `logline-onboarding` | Identity verification and onboarding service |
| `logline-orchestrator` | Service orchestration and management |

## Client Libraries

Client libraries for each service follow a consistent naming pattern:

| Library | Package Name | Main Client Class |
|---------|--------------|------------------|
| ID Client | `logline-id-client` | `LoglineIdClient` |
| Timeline Client | `logline-timeline-client` | `LoglineTimelineClient` |
| Rules Client | `logline-rules-client` | `LoglineRulesClient` |
| Engine Client | `logline-engine-client` | `LoglineEngineClient` |
| Federation Client | `logline-federation-client` | `LoglineFederationClient` |
| API Client | `logline-api-client` | `LoglineApiClient` |

## Class Naming

Core service classes follow the CamelCase convention with the "Logline" prefix:

| Service | Main Service Class | Example |
|---------|-------------------|---------|
| ID Service | `LoglineIdService` | `let service = LoglineIdService::new(config);` |
| Timeline Service | `LoglineTimelineService` | `let service = LoglineTimelineService::new(config);` |
| Rules Service | `LoglineRulesService` | `let service = LoglineRulesService::new(config);` |
| Engine Service | `LoglineEngine` | `let engine = LoglineEngine::new(config);` |

## File Naming

### Documentation Files

Documentation files use kebab-case:

- `architecture.md` - System architecture documentation
- `modularization-plan.md` - Plan for modularizing the system
- `railway-deployment.md` - Railway deployment instructions
- `vision.md` - System vision and philosophy
- `logline-id-service.md` - ID service documentation
- `logline-timeline-service.md` - Timeline service documentation

### Repository README Files

Each service repository has a standardized README file:

- `logline-id/README.md`
- `logline-timeline/README.md`
- `logline-rules/README.md`
- etc.

## API Endpoints

API endpoints follow RESTful conventions with kebab-case:

- `/api/v1/identities`
- `/api/v1/spans`
- `/api/v1/rules`
- `/ws/v1/identity`

## Environment Variables

Environment variables use SCREAMING_SNAKE_CASE:

- `DATABASE_URL`
- `LOGLINE_ID_URL`
- `TIMELINE_SERVICE_PORT`

## GitHub Repositories

GitHub repositories match the service names exactly:

- `github.com/logline/logline-core`
- `github.com/logline/logline-id`
- `github.com/logline/logline-timeline`
- etc.

## Railway Service Names

Railway services match the GitHub repository names:

- `logline-id`
- `logline-timeline`
- `logline-rules`
- etc.

## Migration Guidance

When migrating code from the monolithic architecture to the microservices architecture:

1. Rename modules following the conventions above
2. Update class names to include the `Logline` prefix
3. Standardize client library names and interfaces
4. Convert existing documentation to follow the kebab-case file naming convention

By following these naming conventions consistently, we ensure that the LogLine system remains easy to understand, navigate, and maintain as it grows.