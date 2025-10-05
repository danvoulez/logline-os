


---

<div align="center">

<img src="https://raw.githubusercontent.com/danvoulez/UniverseLogLine/main/.github/logline-logo.png" alt="LogLine Universe Logo" width="175"/>

# 🧭 The LogLine Universe

**An open, computable, and distributed system of record for institutional memory.**

[![Project Status: Production Ready](https://www.repostatus.org/badges/latest/active.svg)](https://www.repostatus.org/#active)
[![Rust CI/CD](https://github.com/logline/logline/actions/workflows/rust.yml/badge.svg)](https://github.com/logline/logline/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Join the Discussion](https://img.shields.io/badge/discussion-GitHub-green.svg)](https://github.com/logline/logline/discussions)

</div>

---

### **Table of Contents**
1.  [The Premise: A Crisis of Record](#-the-premise-a-crisis-of-record)
2.  [The Solution: The Four Pillars of LogLine](#-the-solution-the-four-pillars-of-logline)
3.  [Our Foundational Compact](#-our-foundational-compact)
4.  [Core Concepts: The Vocabulary of a New System](#-core-concepts-the-vocabulary-of-a-new-system)
5.  [System Architecture: A Universe of Services](#-system-architecture-a-universe-of-services)
6.  [Why Rust? The Bedrock of Trust](#-why-rust-the-bedrock-of-trust)
7.  [Getting Started: Launch Your Own Universe](#-getting-started-launch-your-own-universe)
8.  [Use Cases: What You Can Build](#-use-cases-what-you-can-build)
9.  [Project Status & Roadmap](#-project-status--roadmap)
10. [Contributing: Join the Mission](#-contributing-join-the-mission)
11. [License](#-license)

---

## 📖 The Premise: A Crisis of Record

We live in an age of digital amnesia.

Our institutions—governments, companies, collaborations—make critical decisions scattered across ephemeral chats, siloed spreadsheets, and disconnected software. The context of *why* something was done is lost the moment it is done. Artificial intelligences, powerful as they are, operate as stateless oracles, devoid of accountability and a coherent memory of their own actions. The result is a world of institutional black boxes, where provenance is untraceable, decisions are irreversible, and coherence is an illusion.

We can no longer confidently answer the most fundamental questions:
*   **What was truly done?**
*   **By whom, and with what authority?**
*   **Based on what information?**
*   **What were the consequences?**

**LogLine was born in this vacuum.** It is a direct response to this crisis of institutional memory.

## 🏛️ The Solution: The Four Pillars of LogLine

LogLine is a foundational technology for building systems that remember. It provides the infrastructure to create a single, immutable, and computable source of truth for any organization. Its design rests on four pillars:

| Pillar | Description |
| :--- | :--- |
| 1. **Immutable, Verifiable Ledger** | At its heart, LogLine is a **Timeline**—an append-only, chronologically-ordered log of every meaningful action. Every entry is cryptographically signed, creating a tamper-evident chain of events that serves as the ultimate system of record. |
| 2. **Sovereign Computable Identity** | Every actor in the system—be it a human, a service, an AI, or an entire organization—is assigned a **`LogLine ID`**. This is not just a username; it is a cryptographic identity that signs every action, providing undeniable proof of authorship and accountability. |
| 3. **Executable Institutional Logic** | Rules and policies are not passive documents stored in a wiki. They are **`.lll` Contracts**—living, computable files that the system can read, interpret, simulate, and enforce. This transforms governance from a bureaucratic process into an automated, verifiable reality. |
| 4. **Distributed & Federated by Design** | LogLine is not a monolithic, centralized database. It is a **Universe** of interconnected nodes that can operate autonomously, synchronize their timelines, and form federations of trust. This ensures resilience, scalability, and data sovereignty. |

## 📜 Our Foundational Compact

This project is guided by a simple, powerful commitment, which is encoded into its very architecture:

> *Tudo que for feito, será registrado.*<br/>
> *Tudo que for registrado, poderá ser explicado.*<br/>
> *Tudo que for explicado, poderá ser revisto.*<br/>
> *Tudo que for revisto, poderá ser refeito.*<br/>
> *E tudo que for refeito, será parte de uma história viva, legítima e computável.*
>
> ---
>
> *Everything done will be recorded.*<br/>
> *Everything recorded can be explained.*<br/>
> *Everything explained can be reviewed.*<br/>
> *Everything reviewed can be redone.*<br/>
> *And everything redone will be part of a living, legitimate, and computable history.*

## 🔑 Core Concepts: The Vocabulary of a New System

To understand LogLine, you must understand its core primitives:

*   **The `Span`**: The quantum of action. A `span` is a structured, JSON-based data packet containing the who, what, when, and why of a single event. It is the fundamental building block of the Timeline, signed by its author's `LogLine ID`.
*   **The `Timeline`**: The fabric of reality for a LogLine system. It is the immutable, append-only sequence of all `spans`, stored durably in PostgreSQL. It represents the complete, verifiable history of the institution.
*   **The `LogLine ID`**: The signature of sovereignty. A `LogLine ID` is a unique, decentralized identity backed by an Ed25519 keypair. It is the root of all trust and accountability within the Universe.
*   **The `.lll` Contract**: Codified institutional logic. A `.lll` file is a human-readable, machine-executable contract that defines workflows, rules, and state transitions. The `logline-engine` uses these contracts to validate actions before they become `spans` on the Timeline.

## 🌌 System Architecture: A Universe of Services

LogLine is built as a modular Rust workspace, with each core function isolated into its own microservice. This ensures scalability, resilience, and maintainability.

```mermaid
graph TD
    subgraph External World & Clients
        direction LR
        CLI[LogLine CLI]
        WebApp[Web Application]
        SDK[Client SDKs]
    end

    subgraph Platform Boundary
        direction TB
        API_Gateway[logline-api<br/>(API Gateway)]

        subgraph "Real-time Communication Mesh (WebSocket & REST)"
            Engine[logline-engine<br/><strong>Central Orchestrator</strong>]
            Rules[logline-rules<br/><em>Contract Validator</em>]
            Timeline[logline-timeline<br/><em>Ledger Service</em>]
            ID[logline-id<br/><em>Identity Provider</em>]
        end

        subgraph "Shared Persistence Layer"
            Postgres[(PostgreSQL)]
            Redis[(Redis)]
        end
    end

    CLI & WebApp & SDK -- REST/GraphQL --> API_Gateway
    API_Gateway --- Engine

    Engine <--> ID
    Engine <--> Rules
    Engine <--> Timeline

    ID ---> Postgres
    Timeline ---> Postgres
    Rules ---> Postgres
    Engine ---> Redis
```

### Anatomy of a Request: A Span's Journey

1.  **Submission**: A user, via the **CLI** or an **SDK**, initiates an action. The request, containing the action's payload and intent, is sent to the **`logline-api` Gateway**.
2.  **Authentication**: The Gateway authenticates the request using the sender's credentials, verifying them against the **`logline-id`** service. Every request must be tied to a valid, sovereign identity.
3.  **Orchestration**: The authenticated request is passed to the **`logline-engine`**, the brain of the system.
4.  **Validation**: The Engine fetches the relevant **`.lll` Contract** from the **`logline-rules`** service. It simulates the action against the contract's logic to ensure it is valid and permissible.
5.  **Creation**: If validation succeeds, the Engine constructs a new `span`. It populates the `span` with a timestamp, a unique ID, the action's payload, and references to the authorizing contract and workflow.
6.  **Signing**: The Engine uses the author's private key material (managed by the **`logline-id`** service) to cryptographically sign the `span`, creating an undeniable link between the identity and the action.
7.  **Commitment**: The signed `span` is sent to the **`logline-timeline`** service. The Timeline service performs a final integrity check and appends the `span` to the immutable ledger in **PostgreSQL**.
8.  **Receipt**: A confirmation, including the final `span` ID and its hash, is returned through the layers to the original client, serving as a verifiable receipt of their action.

## 🦀 Why Rust? The Bedrock of Trust

A system designed to be an immutable, verifiable source of truth demands a technology stack that is, itself, trustworthy. Rust was chosen for several critical reasons:

*   **Memory Safety without a Garbage Collector**: Rust's ownership model guarantees memory safety at compile time, eliminating entire classes of critical bugs (buffer overflows, null pointer dereferences) that could compromise a system of record.
*   **Performance**: For a system designed to log every action, performance is paramount. Rust provides C++-level speed, ensuring that the act of recording history does not become a bottleneck.
*   **Concurrency**: LogLine is a distributed, multi-service system. Rust's fearless concurrency allows us to build highly parallel and resilient network services without data races.
*   **A Culture of Correctness**: The Rust compiler and ecosystem are built around a philosophy of correctness and explicitness, which aligns perfectly with LogLine's mission to create clear, unambiguous records.

## 🚀 Getting Started: Launch Your Own Universe

You can bootstrap the entire LogLine platform on your local machine using Docker. This provides a fully containerized, isolated environment with all necessary services and databases.

### Prerequisites
*   [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
*   [Docker](https://www.docker.com/get-started/) and Docker Compose
*   A `git` client

### First contact: computable onboarding

The `logline` CLI now automates the full onboarding ritual that wires a new
person, tenant and initial application into the universe. With the gateway
running you can execute the canonical flow:

```bash
logline create identity --name "Daniel Amarilho" --handle dcamarilho
logline create tenant --name "VoulezVous"
logline assign identity dcamarilho --to tenant voulezvous
logline init app --template minicontratos --owner dcamarilho
logline declare purpose --app minicontratos \
  --description "Registrar ações computáveis da empresa VoulezVous"
logline run shell -c "quero registrar um pagamento de €50 para Rafa, feito ontem"
```

Each command calls the new onboarding REST endpoints exposed by
`logline-gateway`, creates timeline spans, issues a JWT bound to the freshly
minted LogLine ID and persists the session locally (in `~/.logline/sessions`).

### Step 1: Clone the Repository
```bash
git clone https://github.com/logline/logline.git
cd logline
```

### Step 2: Configure Your Local Environment
Copy the example environment file. The defaults are configured for the Docker Compose setup and require no changes to get started.
```bash
cp .env.example .env
```

### Step 3: Launch the Universe
This single command will build the Docker images for each microservice, pull the required database images, create a dedicated Docker network, and start all containers in the correct order.
```bash
docker compose up --build -d
```
This process might take a few minutes on the first run as it compiles the Rust services.

### Step 4: Verify the System is Live
Once the command completes, you can verify that all components are running and healthy.

```bash
# Check the status of all containers (should all be 'running' or 'healthy')
docker compose ps

# Ping the health check endpoint for each service
curl http://localhost:8079/health && echo " logline-id is OK"
curl http://localhost:8080/health && echo " logline-timeline is OK"
curl http://localhost:8081/health && echo " logline-rules is OK"
curl http://localhost:8082/health && echo " logline-engine is OK"
# Expected output: {"status":"ok"} for each service.
```

### Step 5: Explore Your Universe
Your local LogLine Universe is now fully operational.
*   **View Logs**: See the real-time output of a service: `docker compose logs -f logline-engine`
*   **Connect to the Database**: Inspect the timeline directly:
    ```bash
    docker compose exec postgres psql -U logline -d logline
    # \dt (to list tables)
    # SELECT * FROM timeline_spans;
    ```
*   **Start Interacting**: Use the (forthcoming) LogLine CLI or build an application with the SDKs to start creating `spans` and building your own institutional memory.

### Step 6: Iterate Quickly (Hot Reload & Hybrid Mode)

*   **Container hot reload**: `docker compose watch logline-engine` ativa o sync automático de arquivos `.rs` para o container e recompila somente quando `Cargo.toml` muda.
*   **Rodar nativamente com infra containerizada**: o script `scripts/dev-hybrid.sh` liga Postgres/Redis/rules/timeline via Docker e executa o serviço selecionado com `cargo run`.
    ```bash
    scripts/dev-hybrid.sh logline-engine
    scripts/dev-hybrid.sh logline-timeline
    ```
    Ideal para usar o IDE local, `cargo watch` e outras ferramentas da linha de comando sem abrir mão do ecossistema já provisionado.

## 🎯 Use Cases: What You Can Build

LogLine is not an application; it is a foundation. Here are a few examples of the systems you can build with it:

*   **Transparent Governance**: Build a voting or proposal system for a DAO or cooperative where every vote is a signed `span`, every proposal is an executable `.lll` contract, and the entire history of decision-making is publicly auditable on the Timeline.
*   **Accountable AI Systems**: Wrap a Large Language Model in a service that forces it to operate with a `LogLine ID`. Every prompt, every generated response, and every action it takes becomes a signed `span`, creating a complete, auditable memory of the AI's behavior and decision-making process.
*   **Reproducible Scientific Research**: Capture an entire experimental workflow—from data ingestion, to parameter changes, to simulation runs—as a chain of `spans`. The entire experiment becomes a verifiable and re-executable object on the Timeline, solving the crisis of reproducibility in science.
*   **Verifiable Supply Chains**: Track a product from origin to consumer. Every step—harvest, processing, shipping, delivery—is recorded as a `span` on a shared Timeline, signed by the responsible party, creating an unbreakable chain of custody.

## 🗺️ Project Status & Roadmap

The LogLine Universe is an active, open project on a clear path to becoming a new standard for verifiable systems.

-   ✅ **Phase 1: Foundation (100%)** - Core libraries and foundational services (`id`, `timeline`) are complete and stable.
-   ✅ **Phase 2: Core Logic (100%)** - The `rules` and `engine` services are functional, enabling contract-based execution.
-   🔄 **Phase 3: Integration & APIs (In Progress)** - We are currently building the `logline-api` gateway and the first set of client SDKs.
-   ⏳ **Phase 4: Production Hardening (Next Up)** - Focus will shift to comprehensive CI/CD, advanced security protocols, and sophisticated monitoring.

For a granular view of our progress and future plans, please consult our public [**Project Roadmap**](./ROADMAP.md) and [**Task List**](./TASKLIST.md).

## 🤝 Contributing: Join the Mission

LogLine is more than just code; it is a movement to build more transparent, accountable, and coherent systems. We invite developers, architects, ethicists, and visionaries to join us.

There are many ways to contribute:
*   **Code**: Help build the core services, the API gateway, or client SDKs.
*   **Documentation**: Improve this README, write tutorials, or document our APIs.
*   **Examples**: Build example applications using `.lll` contracts to showcase what's possible.
*   **Ideas**: Participate in [discussions](https://github.com/logline/logline/discussions) to shape the future of the platform.

To get started, please read our (forthcoming) `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md` guides.

---

## 📄 License

The LogLine Universe is open-source software licensed under the **MIT License**. See the [LICENSE](./LICENSE) file for details.

<div align="center">

**Join us in building a more verifiable world.**

</div>
