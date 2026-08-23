# Distributed Payment Platform — Component-Wise Build Plan
 
A single Rust-based, event-sourced payment platform, laid out below **in the order you should build it**. Nine core components (1–9) cover essentially everything that matters for Juspay onboarding and backend/system-design interviews. Three more are genuinely optional — build them only if time remains, in the priority order given, or skip them entirely and the project still stands on its own.
 
Total estimated code: **~4,100–5,850 lines** of Rust on the core path, up to **~6,900** if all three optional components are also built, plus ~500–700 lines of config/YAML, across 8 weeks at 10–15 hrs/week.
 
---
 
## Component 1 — Requirements & Capacity Planning
 
**Goal:** Before writing any code, do the exercise almost everyone skips: turn "build a payment platform" into numbers. This is a design doc, not code, and it's the single highest-leverage hour of the whole project for interviews — it's exactly what a system design round tests, and most candidates have never actually done it for something they built.
 
**What you'll be able to do:**
- State functional requirements (what the system does) and non-functional requirements (latency budget, availability target, consistency requirements) for the payment platform in writing.
- Do back-of-envelope math: assume a QPS (e.g. 500 payments/sec peak), derive storage growth per year, Kafka throughput needed, DB connections needed, and cache size to hit a target hit-rate — then use these numbers to justify every later design choice (why sharding kicks in, why you need read replicas) instead of adding them because the roadmap said to.
- Defend every number out loud — "why 500 QPS and not 5,000" — because that's the actual interview.
**How you'll do it:**
- One document: functional reqs (payments, refunds, balance queries), non-functional reqs (p99 latency target, 99.9% availability target, RPO/RTO if a node dies).
- Estimation math: pick a realistic scale (e.g. mid-size PSP), compute QPS → daily/yearly storage → bandwidth → cache sizing, using round numbers and explicit assumptions.
- Revisit and correct this doc once mid-project and once at the end — your first estimates will be wrong, and *noticing why* is the actual skill.
**Estimated LOC:** 0 (this is a 1–2 page markdown doc, not code)
 
**Technologies:** none — pen/paper or markdown
 
**What you'll learn:** The requirements-and-estimation framework itself (concept #1, the one tagged Deep for a reason), and — critically — a document you can literally hand an interviewer that shows you can reason about scale before touching a keyboard.
 
---
 
## Component 2 — Payment Service
 
**Goal:** The transactional core. Accepts a payment request, guarantees it's processed exactly once even under client retries, and persists it durably.
 
**What you'll be able to do:**
- `POST /payments` with an `Idempotency-Key` header — retry it 10 times, get the same result, never double-charge.
- `GET /payments/{id}` to check status.
- Run two concurrent requests with the same idempotency key and watch only one actually execute.
**How you'll build it:**
- Axum for the HTTP layer, `sqlx` for Postgres (compile-time checked queries).
- An `idempotency_keys` table: key → request hash → response body → status, with a unique constraint doing the concurrency-safety work for you.
- Redis as a fast-path lock (`SET key NX EX 30`) in front of the Postgres check, so concurrent duplicate requests don't both race into the DB.
- A `payments` table with a state machine: `pending → processing → succeeded/failed`, transitions guarded by a DB transaction at `SERIALIZABLE` or `REPEATABLE READ` isolation (you'll deliberately break it at `READ COMMITTED` once, to *see* the anomaly before fixing it).
**Estimated LOC:** ~900–1,200 (handlers ~250, DB layer ~350, idempotency middleware ~200, tests ~300–400)
 
**Technologies:** Rust, Axum, sqlx, Postgres, Redis, tokio
 
**What you'll learn:** Ownership/borrowing under real async load, `Result`-based error handling end-to-end, transaction isolation levels (with a bug you caused yourself as proof), Redis as a lock — not just a cache, and why idempotency is genuinely harder than "don't send duplicates."
 
---
 
## Component 3 — API Gateway
 
**Goal:** One entry point in front of Payment Service (and, later, Ledger Service), exposed multiple ways so you get hands-on with each.
 
**What you'll be able to do:**
- Call the same underlying data via a REST endpoint and a GraphQL query, and see HATEOAS `_links` in the REST response telling the client what it can do next (e.g. a pending payment's response links to a `cancel` action).
- Authenticate with a JWT, get a 401 on expiry, refresh it.
- Get live payment status updates over an SSE stream instead of polling.
**How you'll build it:**
- `async-graphql` crate wrapping the same service calls Payment Service already exposes — this is mostly plumbing once the core service exists, which is the point: GraphQL as a thin aggregation layer, not a rewrite.
- Middleware for JWT validation (`jsonwebtoken` crate).
- A small `HypermediaResponse<T>` wrapper type that adds a `links: Vec<Link>` field based on the resource's current state — this is the whole of HATEOAS in practice, and it's a genuinely small pattern once you've written it once.
- A `/payments/{id}/events` SSE endpoint streaming status transitions as they happen — a few dozen lines with Axum's SSE support.
**Estimated LOC:** ~550–800 (REST routes ~150, GraphQL schema/resolvers ~200, auth middleware ~150, SSE endpoint ~50)
 
**Technologies:** Rust, Axum, async-graphql, jsonwebtoken
 
**What you'll learn:** GraphQL resolver design and the N+1 problem, JWT/OAuth2 basics end-to-end (not just "add a header"), what HATEOAS is actually for — letting clients discover valid transitions instead of hardcoding them — and real-time push vs polling.
 
---
 
## Component 4 — Kafka Event Bus + Saga Orchestration
 
**Goal:** Decouple the services — Payment Service shouldn't call downstream services directly and block on them. It publishes an event; interested services react.
 
**What you'll be able to do:**
- Kill a downstream consumer mid-flight, restart it, and watch it catch up from the last committed Kafka offset with no lost events.
- Trace one payment's saga end-to-end through logs: `PaymentInitiated → FundsReserved → LedgerUpdated → NotificationSent`.
- Force a failure partway through the saga and watch the compensating action run (e.g. release the reservation).
**How you'll build it:**
- `rdkafka` crate, one topic per event type (or a single `payment-events` topic with a type field — simpler to start).
- Payment Service becomes a saga orchestrator: on each step's completion event, it decides and publishes the next command; on failure, it publishes a compensating command.
- Consumer groups for Reconciliation and Notification so each independently tracks its own offset.
**Estimated LOC:** ~400–600 (producer wrapper ~100, consumer wrapper ~150, saga state machine ~200)
 
**Technologies:** Rust, rdkafka, Kafka (via Docker), Postgres (for saga state persistence)
 
**What you'll learn:** Why exactly-once isn't free (you'll implement idempotent consumers to get it), consumer groups and partition assignment, and the Saga pattern as the answer to "how do you do a transaction across services without 2PC."
 
---
 
## Component 5 — Ledger Service (CQRS + Event Sourcing + DDD)
 
**Goal:** The most conceptually dense component, and the one most worth doing properly. Instead of storing "current balance" as a mutable row, you store every balance-changing fact as an immutable event, and derive balance from replaying them. Built now, once Kafka exists, so its events have somewhere real to flow from.
 
**What you'll be able to do:**
- Append a `FundsDebited` / `FundsCredited` event and see the ledger's read-side balance update.
- Rebuild a merchant's balance from scratch by replaying only their event stream — and get the identical number every time.
- Deliberately corrupt the read model, then regenerate it from the event log, proving the events are the source of truth, not the read table.
**How you'll build it:**
- Define the bounded context first, on paper: what's a `Merchant` aggregate, what events can it emit, what invariants must hold (balance can't go negative, etc.) — this is the DDD part, and skipping it is why most people's "event sourcing" projects are just an audit log with extra steps.
- An `events` table (append-only, `aggregate_id`, `event_type`, `payload`, `version`) — this is your write model.
- A projector: a Kafka consumer that reads new payment events and updates a `balances` read table — this is your CQRS read side, intentionally eventually-consistent.
- Optimistic concurrency via an event `version` column, so two concurrent writers to the same aggregate can't silently clobber each other.
**Estimated LOC:** ~700–1,000 (aggregate/domain logic ~300, event store ~200, projector ~200, replay tool ~150)
 
**Technologies:** Rust, Postgres (as the event store — no need for a specialized one), serde for event serialization
 
**What you'll learn:** What a bounded context actually constrains (not just a folder name), why CQRS's split exists (read and write have different scaling/consistency needs), what "eventually consistent" costs you in practice, and optimistic concurrency as an alternative to locking.
 
---
 
## Component 6 — Reconciliation Worker
 
**Goal:** A batch job that periodically compares Payment Service's view of the world against Ledger Service's, catching the drift that eventual consistency guarantees will occasionally produce. Needs both of the previous two components to exist first, which is why it's built now.
 
**What you'll be able to do:**
- Run it on demand, get a report of any mismatched transactions.
- Watch it correctly process a batch of thousands of records concurrently, bounded by a worker pool so you don't open 10,000 DB connections at once.
**How you'll build it:**
- Tokio task spawning with a bounded `Semaphore` to cap concurrency.
- A simple diff: pull both tables, compare by transaction ID, flag mismatches.
- Scheduled via a cron-like loop or a simple `tokio::time::interval`.
**Estimated LOC:** ~300–500
 
**Technologies:** Rust, tokio (`Semaphore`, `JoinSet`), Postgres
 
**What you'll learn:** Bounded concurrency patterns (the producer-consumer pattern from your original roadmap, for real this time), and why reconciliation and idempotency are two different, both-necessary safety nets.
 
---
 
## Component 7 — Infra: Docker → Kubernetes + Service Mesh + Observability
 
**Goal:** Take everything above from "runs on my laptop" to "deploys like a real distributed system," and make it debuggable when something goes wrong across multiple services.
 
**What you'll be able to do:**
- `docker compose up` and have all core services + Postgres/Redis/Kafka come up together.
- Apply Kubernetes manifests and see the same system running as pods, with at least two services talking through an Istio sidecar so you've seen mTLS and traffic policy applied, not just described.
- Trace one request across services in Jaeger/Grafana using a single correlation ID.
- Push a commit and watch a CI pipeline run tests and build the Docker images automatically.
**How you'll build it:**
- `docker-compose.yml` first — this is your everyday dev loop.
- K8s manifests (or a Helm chart) as a second deployment target, not a replacement — deploy 2–3 core services to K8s to prove you can, rather than migrating everything.
- Istio (or Linkerd) sidecars on two services for the service-mesh piece — mTLS between them, a simple traffic-split demo.
- `tracing` crate + OpenTelemetry exporter → Jaeger; Prometheus metrics via `metrics` crate → Grafana dashboard with p50/p95/p99.
- A GitHub Actions workflow: `cargo test` + `cargo clippy` + `docker build` on every push — a few dozen lines of YAML, not a project on its own.
- Twelve-Factor audit pass at the end: config via env vars only, stateless processes, logs as event streams — go through the checklist against what you already built and fix violations rather than designing for it upfront.
**Estimated LOC/config:** ~150–250 lines Rust (tracing/metrics instrumentation) + ~350–550 lines YAML (compose, k8s manifests, Istio config, CI workflow)
 
**Technologies:** Docker, Docker Compose, Kubernetes, Istio, tracing/OpenTelemetry, Jaeger, Prometheus, Grafana, GitHub Actions
 
**What you'll learn:** The actual difference between logs/metrics/traces (and why p99 matters more than average), what a service mesh buys you beyond "another YAML file," the Twelve-Factor principles as things you retrofit and understand rather than a checklist you memorize, and a CI pipeline that actually gates your own commits.
 
---
 
## Component 8 — Scaling & Resilience Lab
 
**Goal:** Everything in this component takes a service you've *already built* and either multiplies it (to get load balancing, replication, sharding, quorum, leader election) or breaks it on purpose (to get CAP/PACELC/network-fallacies as something you watched happen, not just read about). This is the component that turns "I built a payment platform" into "I can talk through how it scales" — the actual difference in a system design interview.
 
It's structured as independent sub-labs — do as many as time allows, in any order, each is a half-day to one weekend:
 
**a) Load balancing.** Run 3 replicas of Payment Service behind Nginx doing round robin, hit it, confirm even distribution via logs. Then replace it with a small hand-rolled consistent-hash router (hash merchant ID → instance) in front of the Ledger read replicas from (b), and add/remove an instance to see how few keys remap compared to naive modulo hashing. *(concepts 4, 3)*
 
**b) Database replication.** Add 2 Postgres read replicas for the Ledger read model; Payment Service writes to primary, a new `GET /balance` reads from a replica. Deliberately write, then immediately read from a replica before it catches up, and observe the stale read — screenshot it, this is replication lag as a real bug you'll debug in production one day. *(concepts 10, 7)*
 
**c) Database sharding.** Shard the Ledger event store by `hash(merchant_id) % N` across 2 Postgres instances, with a small shard router in front. Write up (no need to implement) how range-based and directory-based sharding would differ for this same data, and when you'd pick each over hash-based. *(concept 9)*
 
**d) Quorum reads/writes.** Depends on Optional Component C (Cassandra) being built. Configure it with `QUORUM` consistency instead of the default, kill one node in the cluster, confirm reads/writes still succeed; then set `ALL` and watch it fail the same way. *(concept 24)*
 
**e) Leader election.** Run 3 replicas of the Reconciliation Worker; only one should run the batch job at a time. Implement via a Postgres advisory lock or a Redis lease (whichever holds the lock is leader), log who's leader, kill that instance, and watch a new leader take over within seconds. *(concept 25, ties back into 26 which you already have)*
 
**f) Chaos / network partition.** Use `docker network disconnect` to sever Payment Service from Ledger Service mid-flight. Document what actually happens — does Payment Service stay available and queue the event (AP-ish) or block (CP-ish)? Do the same for Payment Service ↔ its Postgres primary. Write the answers up explicitly against CAP and PACELC — "if partitioned, X; else, latency vs consistency tradeoff is Y." This is the smallest, single-dependency version of failure testing — Component 9 runs the full set once more of the system exists. *(concepts 5, 6, 23)*
 
**g) Message broker comparison.** Re-wire just the Notification path to use RabbitMQ (fanout exchange) instead of Kafka, alongside the existing Kafka paths elsewhere. Now you've operated both, and can write a short, honest comparison instead of reciting one. *(concept 17, deepens 16/18)*
 
**h) Synchronous RPC.** Add a gRPC endpoint from Payment Service to Ledger Service for a synchronous "check balance before reserving funds" call, alongside the existing async Kafka flow. You'll now have used REST, GraphQL, gRPC, and async messaging firsthand, plus SOAP if you build that optional component too. *(concept 20)*
 
**i) Rate limiting + cache depth.** Add a Redis token-bucket rate limiter on the API Gateway (429 on burst). Extend the existing cache-aside endpoint with a second one using write-through, add jittered TTLs, and add a Redis lock around cache misses so a stampede of requests on a cold key doesn't all hit Postgres at once. *(concepts 21, 14, 15, 12 — the denormalized read table you already built in Component 5 covers 12)*
 
**j) Service discovery.** If you deploy to Kubernetes in Component 7, this is close to free — services already find each other via K8s DNS; just point it out explicitly and, if you have time, compare against manually registering with Consul for one service. *(concept 28)*
 
**Estimated LOC:** ~600–900 (load balancer/consistent-hash router ~150, shard router ~150, leader election ~100, gRPC endpoint ~100, rate limiter ~80, RabbitMQ path ~100, write-ups are not code)
 
**Technologies:** Nginx, Postgres replication, Redis (advisory locks/leases), RabbitMQ, gRPC (tonic), Docker networking
 
**What you'll learn:** This is the component where the concepts stop being definitions. You'll have *watched* a stale replica read, *watched* a leader failover, *watched* a consistent-hash rebalance touch almost nothing — which is the difference between reciting CAP theorem and being able to argue about it in an interview.
 
---
 
## Component 9 — Load Testing & Chaos Engineering
 
**Goal:** Everything so far was built and tested one request at a time. This component finds out what actually happens under real concurrent load — not benchmarked in your head, measured — and what actually happens when a dependency dies, not assumed. Build this after Component 7 (you need the dashboards to see anything) and roughly alongside or after Component 8 (you need multiple instances/replicas for the chaos experiments to mean something). Arguably the highest-ROI component per hour spent.
 
**What you'll be able to do:**
- Run 500 / 1,000 / 5,000 concurrent requests against the API Gateway and read off p50/p95/p99 latency, throughput, CPU/memory, DB connection pool saturation, Redis hit rate, and Kafka consumer lag from the Grafana dashboards you already built.
- Find the actual point where the system falls over (connection pool exhaustion, a missing index, a lock you didn't know was contended) and fix it, then re-run and see the number move.
- Kill Redis, Kafka, and the Postgres primary one at a time under load and document exactly what breaks, what degrades gracefully, and what recovers on its own vs needs intervention.
- Send a duplicate webhook and a deliberately slow/timeout webhook from the mock bank adapter and confirm your retry/timeout handling does what you think it does.
**How you'll build it:**
- `goose` (a Rust-native load-testing framework — fitting, given the rest of the stack) or `k6`/`wrk` from outside, scripted to hit realistic payment flows, not just one endpoint.
- One experiment at a time: form a hypothesis first ("I think we can handle 2,000 concurrent before p99 exceeds 500ms"), run it, record what actually happened, fix the bottleneck, re-run.
- Chaos via `docker kill` / `docker pause` on Redis/Kafka/Postgres containers, `tc` (Linux traffic control) or toxiproxy for injecting slow/flaky network conditions on the mock bank webhook.
- A short written report per experiment — hypothesis, result, root cause, fix, re-test — this report is arguably more resume-worthy than the load test scripts themselves.
**Estimated LOC:** ~150–300 Rust (goose load scripts) + toxiproxy/chaos config, no new production code — this component consumes and stress-tests what you've already built
 
**Technologies:** goose (or k6), the Grafana/Prometheus/Jaeger stack from Component 7, toxiproxy, Docker chaos commands
 
**What you'll learn:** How "know the theory" turns into "measured the number myself" — connection pooling, index usage, lock contention, queue depth, and backpressure stop being vocabulary and become things you caused, watched, and fixed.
 
---
 
## Optional components
 
Build these only if time remains after Component 9, roughly in this priority order. Each is an independent add-on off the Kafka bus — none of them are prerequisites for anything in the core path, except where noted.
 
### Optional A — Legacy Bank Adapter (SOAP)
 
**Goal:** Simulate the unglamorous reality of payments engineering — most banks you'll integrate with expose SOAP, not REST. This is one weekend, not a rabbit hole.
 
**What you'll be able to do:** Call a REST endpoint on your side, have it translate to a SOAP envelope, hit a mock SOAP bank endpoint, and translate the response back.
 
**How you'll build it:** Hand-roll the SOAP envelope as an XML template (`quick-xml` or plain string templating is enough) and a tiny mock "bank" service (a second Axum app) that accepts the SOAP XML and returns a canned XML response.
 
**Estimated LOC:** ~250–400 · **Technologies:** Rust, quick-xml or reqwest + string templates
 
**What you'll learn:** What a SOAP envelope actually looks like under the REST abstractions you're used to, and enough to not be lost in a legacy-integration meeting.
 
### Optional B — Search Service (Elasticsearch)
 
**Goal:** Let a support/ops person search transactions by free text (merchant name, partial card reference, error message) — the kind of query Postgres `LIKE` handles badly at scale.
 
**What you'll be able to do:** Search "failed payments from merchant X last week mentioning timeout" and get relevant results ranked, not just exact matches.
 
**How you'll build it:** A Kafka consumer indexing payment events into an Elasticsearch index, one well-designed mapping (`keyword` vs `text` matters), and a `/search` endpoint on the gateway proxying to Elasticsearch's query DSL.
 
**Estimated LOC:** ~200–350 · **Technologies:** Rust, elasticsearch crate (or plain reqwest + JSON), Elasticsearch (Docker)
 
**What you'll learn:** Inverted indexes at an intuitive level, mapping design, and why full-text search is a genuinely different tool from a DB index, not just "grep but hosted."
 
If you're short on time, better Kafka work, better concurrency, and better test coverage on the core services teach more than this does — feel free to skip it.
 
### Optional C — Audit Log Service (Cassandra)
 
**Goal:** An append-only, extremely high-write log of every action in the system, using a store built for exactly that write pattern instead of stressing Postgres with it. Build this last if at all — until you've felt Postgres strain under a high-write append-only load, Cassandra just looks like "another database with different syntax"; the appreciation only lands after the contrast.
 
**What you'll be able to do:** Fire a burst of events and watch write latency stay flat as volume grows, in a way Postgres wouldn't at the same insert rate. Query "all actions for transaction X" fast, because you partitioned by transaction ID from the start.
 
**How you'll build it:** A Kafka consumer writing to a Cassandra table partitioned by `transaction_id`, clustered by timestamp. Deliberately design the partition key wrong once (e.g. partition by day instead of transaction ID), hit a hot-partition problem, then fix it.
 
**Estimated LOC:** ~200–350 · **Technologies:** Rust, scylla crate (Cassandra-compatible driver), Cassandra (Docker)
 
**What you'll learn:** Partition key design and why it's the single most consequential decision in a wide-column store, and when "eventually consistent, massively writable" beats ACID.
 
If you build this, Component 8d (quorum reads/writes) becomes available too. If you skip it, you lose that one sub-lab — a Light-depth concept, an acceptable trade against finishing the core path solidly instead.
 
### Cut entirely — not included as optional
 
A **Fraud Graph Service (Neo4j)** was considered and removed: low relevance to Juspay onboarding and most backend interviews relative to the time cost of learning a new database and Cypher for one narrow use case. If it ever comes up in an interview, you can speak to *why* a graph model fits fraud-detection problems without having built one.
 
---
 
## Rough totals
 
| Component | LOC (Rust) |
|---|---|
| 1. Requirements & Capacity Planning | 0 (design doc) |
| 2. Payment Service | 900–1,200 |
| 3. API Gateway | 550–800 |
| 4. Kafka + Saga | 400–600 |
| 5. Ledger Service | 700–1,000 |
| 6. Reconciliation | 300–500 |
| 7. Infra/Observability | 150–250 + ~450 YAML |
| 8. Scaling & Resilience Lab | 600–900 |
| 9. Load Testing & Chaos Engineering | 150–300 |
| **Core total** | **~4,100–5,850 lines Rust** |
| Optional A — SOAP Bank Adapter | 250–400 |
| Optional B — Elasticsearch | 200–350 |
| Optional C — Cassandra | 200–350 |
| **Full total if all optionals built** | **~4,750–6,950 lines Rust** |
 
---
 
## Full concept coverage (your 31-concept list)
 
| # | Concept | Depth | Covered by |
|---|---|---|---|
| 1 | Requirements & estimation framework | Deep | Component 1 |
| 2 | Vertical vs horizontal scaling | Medium | Component 8a, discussed in Component 1 |
| 3 | Stateless vs stateful services | Medium | Component 2 (stateless API) vs Component 5/Optional C (stateful stores); Component 8a |
| 4 | Load balancing algorithms | Medium | Component 8a |
| 5 | CAP theorem | Deep | Component 5 (eventual consistency in practice) + Component 8f |
| 6 | PACELC theorem | Light | Component 8f |
| 7 | Eventual consistency models | Medium | Component 5, Component 8b |
| 8 | SQL vs NoSQL — when to use which | Deep | Postgres (Component 5) vs Optional C (Cassandra) vs Optional B (Elasticsearch), each picked for a stated reason |
| 9 | Database sharding | Deep | Component 8c |
| 10 | Database replication | Deep | Component 8b |
| 11 | Indexing basics | Medium | Component 2 (`EXPLAIN`), Component 5 (event table indexes) |
| 12 | Denormalization for performance | Light | Component 5 (materialized read model) |
| 13 | Caching fundamentals | Light | Component 2 (Redis idempotency/lock) |
| 14 | Cache patterns | Medium | Component 8i |
| 15 | Cache stampede prevention | Light | Component 8i |
| 16 | Message queues — sync vs async, pub-sub | Medium | Component 4 |
| 17 | Kafka vs RabbitMQ | Medium | Component 8g |
| 18 | Delivery guarantees + DLQ | Light | Component 4 (idempotent consumers), Component 8g |
| 19 | REST API design principles | Medium | Component 2, 3 |
| 20 | GraphQL vs gRPC vs REST | Light | Component 3 (GraphQL), Optional A (SOAP), Component 8h (gRPC) |
| 21 | API rate limiting & throttling | Medium | Component 8i |
| 22 | API Gateway pattern | Light | Component 3 |
| 23 | Distributed systems fallacies & partitions | Light | Component 8f |
| 24 | Quorum-based reads/writes | Light | Component 8d (depends on Optional C — Cassandra) |
| 25 | Leader election | Light | Component 8e |
| 26 | Distributed locking | Light | Component 2 (Redis lock), Component 8e |
| 27 | Monolith vs microservices trade-offs | Medium | Whole project's architecture + Component 1 writeup |
| 28 | Service discovery | Light | Component 7 (K8s DNS), Component 8j |
| 29 | Saga pattern | Light | Component 4 |
| 30 | Security basics | Light | Component 3 (JWT/OAuth2), Component 7 (Istio mTLS/TLS) |
| 31 | Observability basics | Light | Component 7 |
 
Every concept on your list maps to a component you'll actually build or an experiment you'll actually run — none of them are "read about it" items.
