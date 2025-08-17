# ✅ RustyGPT — Full Implementation Plan (TODO Checklist)

A privacy-respecting, full-stack AI platform in Rust (Axum + Yew) that integrates local models, PostgreSQL, OAuth (Apple/GitHub), SSE streaming, stored procedures, and AI tools for survival, conversation, and reasoning.

**NOW PRIORITIZING:** Full GitHub Copilot Chat compatibility via OpenAI-compatible `/v1/chat/completions` and `/v1/models` endpoints.

**All LLM inference, model loading, pipeline orchestration, and task execution is handled locally—no external API calls or model services.**

---

## 🧱 0. Minimal AI Chat MVP (Single-User, Local-Only)

**Current State**: Basic endpoints exist but return dummy responses. Need actual local LLM inference integration.

**Goal**: Transform existing dummy chat completions endpoint into a functioning local AI chat with real model inference.

- [X] ~~Use crate: `rustygpt-server`~~ ✅ **Already exists**
  - [X] ~~Axum-based REST API~~ ✅ **Already implemented**
  - [X] ~~`POST /v1/chat/completions` — accepts JSON with `messages` array~~ ✅ **Already exists (dummy response)**
  - [ ] **UPGRADE**: Replace dummy echo responses with actual LLM inference
  - [ ] `stream: true` parameter support in `ChatCompletionRequest`
  - [ ] Connect existing SSE infrastructure to chat completions endpoint
- [ ] **NEW**: Add local model loading to `rustygpt-server`
  - [ ] Add model loading dependencies (`candle-core`, `candle-transformers`, or `llama-rs`)
  - [ ] Load a local GGUF model (e.g., Llama 3.2 3B)
  - [ ] Expose basic streaming and blocking completion interface
  - [ ] Integrate model manager with existing `AppState`
- [X] ~~Add shared types crate: `rustygpt-shared`~~ ✅ **Already exists**
  - [X] ~~OpenAI-compatible request and response types~~ ✅ **Already implemented**
  - [ ] **UPGRADE**: Add `stream` parameter to `ChatCompletionRequest`
  - [ ] Add streaming response types for `delta` format
- [ ] **UPGRADE**: Implement actual model manager in `rustygpt-server`
  - [ ] Load 1 local model on server startup
  - [ ] Run inference and return completion tokens (not echoes)
  - [ ] Handle both streaming and non-streaming requests
- [ ] **UPGRADE**: Connect existing SSE streaming output to chat completions
  - [X] ~~Use `axum::response::sse::Sse`~~ ✅ **Infrastructure exists**
  - [ ] Emit correct OpenAI `delta` format for streaming responses
  - [ ] Route streaming responses through existing SSE infrastructure
- [X] ~~CLI to run API server~~ ✅ **Already works via `rustygpt-cli`**
- [ ] **TEST**: Manual test client to send real completion requests (not just echoes)

**Priority**: Replace echo responses with actual local LLM inference while leveraging existing infrastructure.

---

## 1. Refactor CLI and Server

- [X] Move CLI out of Server
- [X] Add `lib` to Server
- [X] Enable CLI to run Server via `lib`

## 2. Copilot Chat-Compatible API First

- [X] Scaffold `rustygpt-server` crate to serve Copilot-compatible endpoints
  - [X] `POST /v1/chat/completions` with OpenAI schema
  - [X] `GET /v1/models` listing available model(s)
- [X] Static response MVP
  - [X] Return dummy assistant message
  - [ ] Confirm Copilot Chat connects
- [ ] Implement RustyGPT backend model interface
  - [ ] Use internal inference engine (no external Ollama or OpenAI)
  - [ ] Translate OpenAI requests to internal model prompt format
  - [ ] Return model streaming response as SSE chunks
- [ ] Streaming support
  - [ ] `stream: true` handling using `axum::response::sse::Sse`
  - [ ] Emit correct `delta` format per OpenAI spec
- [ ] Token accounting
  - [ ] Integrate `tiktoken-rs` or local equivalent
  - [ ] Return `usage` in completion response
- [ ] Handle extended parameters
  - [ ] Accept and ignore: `logit_bias`, `user`, `frequency_penalty`, etc.
  - [ ] Gracefully skip unsupported params
- [ ] Logging & Debugging
  - [ ] Add `/status` endpoint
  - [ ] Log: model, duration, stream flag, token count
- [ ] Testing
  - [X] Unit + integration tests for `/v1/chat/completions`
  - [ ] Manual validation in GitHub Copilot Chat

---

## 3. Project Structure & Workspace

- [X] Set up `Cargo.toml` workspace with crates
  - [X] `rustygpt-shared` (was `rustygpt-common`)
  - [X] `rustygpt-server` (was `rustygpt-api`)
  - [X] `rustygpt-web` (was `rustygpt-frontend`)
  - [X] `rustygpt-cli`
  - [X] `rustygpt-tools` (was `rustygpt-utils`)
  - [ ] `rustygpt-model` (integrated into server for now)
  - [ ] `rustygpt-db` (integrated into server for now)
  - [ ] `rustygpt-index`
- [X] Add Makefile or justfile
- [ ] Add `.cargo/config.toml` for targets

---

## 4. Backend (Axum)

- [X] Setup Axum API project
- [X] Add OpenAPI support via `utoipa` or `paperclip`
- [X] Define route tree
  - [X] `/auth` (OAuth routes)
  - [X] `/api/conversations` (chat)
  - [ ] `/search`
  - [ ] `/admin`
  - [X] `/v1/chat/completions` (Copilot API)
  - [X] `/v1/models` (Copilot API)
- [X] Define `AppState`
  - [X] Config
  - [X] DB pool
  - [ ] Cache
  - [ ] Model runtime handle
- [X] Add middleware
  - [X] Tracing
  - [X] Auth (JWT / Cookie)
  - [X] CORS
  - [ ] Compression
- [X] Implement SSE streaming endpoint

---

## 5. Authentication & Accounts

- [ ] Local login
  - [ ] Email + password (argon2)
- [X] External auth
  - [X] Apple Sign In
  - [X] GitHub OAuth
- [X] Stored procedures in Postgres
  - [X] Create user
  - [X] Login
  - [X] Validate session
  - [X] Link OAuth identity
- [ ] Token handling
  - [ ] Short-lived JWT
  - [ ] Secure refresh cookie

---

## 6. AI Model Integration (LLM)

- [ ] Implement internal model manager
  - [ ] Load GGUF/ONNX models from disk
  - [ ] Manage tokenizer, config, and metadata
- [ ] Define unified engine trait for chat and embedding tasks
  - [ ] Support stream and blocking modes
  - [ ] Handle concurrency and cancellation
- [ ] Integrate task queues and LLM job controller
- [ ] Implement async model runtime per model backend
- [ ] Add structured logging and retry controls

---

## 7. File, Book, and Knowledge Indexing

- [ ] Create `rustygpt-index`
- [ ] Add parsers
  - [ ] EPUB
  - [ ] Markdown
  - [ ] Plain text
- [ ] Chunk + embed content
- [ ] Store metadata in DB
- [ ] Use local vector DB (`tantivy` or `qdrant`)
- [ ] Watch file directory for changes

---

## 8. Testing & Coverage

- [X] Add `cargo llvm-cov` to workflow
  - [X] Command: `cargo llvm-cov --workspace --html --output-dir .coverage && open .coverage/index.html`
- [X] Write unit tests
  - [X] API routes
  - [X] SSE stream
  - [X] Stored procedures
- [X] Add Yew tests for frontend logic
- [ ] Target 90%+ coverage in all crates

---

## 9. Frontend (Yew)

- [X] Set up Yew project
- [X] Integrate TailwindCSS + Trunk
- [X] Implement pages
  - [X] Auth (Sign In, Sign Up)
  - [X] Chat
  - [ ] Search
  - [ ] Admin
- [X] Build components
  - [X] ChatBox
  - [ ] Streaming tokens
  - [ ] Threaded view
- [X] Use shared types via `rustygpt-shared`

---

## 10. Dockerized Environment

- [X] Define Docker Compose stack
  - [X] `rustygpt-server` (was `rustygpt-api`)
  - [X] `rustygpt-web` (was `rustygpt-frontend`)
  - [X] `postgres`
  - [ ] `rustygpt-model` (internal LLM server)
- [X] Mount volumes
  - [X] `/array/data/books`
  - [X] `/array/data/media`
- [ ] Enable GPU pass-through (REQUIRED For Linux and macOS)

---

## 11. Tooling & Dev Experience

- [X] Add `COPILOT_INSTRUCTIONS.md`
  - [X] Idiomatic Rust enforcement
  - [X] Explicit type links in docstrings
  - [X] No unnecessary clones
  - [X] Crate version synchronization
- [X] Add `dev.md` (via Justfile and README)
  - [X] Common commands
- [X] Add CLI tools or scripts
  - [X] DB seed (via docker-compose)
  - [ ] Model download/init
  - [X] Lint / check / coverage (via Justfile)

---

## 12. Conversation Explorer

- [ ] Message model
  - [ ] `thread_id`
  - [ ] `parent_id`
  - [ ] `message_type`
- [ ] Frontend features
  - [ ] Display message branches
  - [ ] Expand/collapse threads
  - [ ] Move to thread
- [ ] Thread summarization
- [ ] Auto-thread detection on topic shift

---

## 13. Critical Thinking Tools

- [ ] Prompt styles
  - [ ] Socratic
  - [ ] Devil’s Advocate
  - [ ] Chain of Thought
- [ ] Add response rating UI
  - [ ] Confidence sliders
  - [ ] Flag assumptions
- [ ] Classify conversation type
  - [ ] Reflective
  - [ ] Analytical
  - [ ] Practical

---

## 14. Privacy & Offline Use

- [ ] Run all inference locally
- [ ] No cloud dependencies
- [ ] Store all data locally
- [ ] Offer “offline survival mode”

---

## 15. Roles & Permissions

- [ ] Define roles
  - [ ] Admin
  - [ ] User
  - [ ] Read-only
- [ ] Protect endpoints with role-based guards
- [ ] Enforce message access control

---

## 16. Prompt Enrichment & Memory

- [ ] Generate thread summaries
- [ ] Token-bounded memory injection
- [ ] Enrich prompts with:
  - [ ] Glossary
  - [ ] System instructions
  - [ ] Relevant prior messages

---

## 17. RustyGPT Image Generation

- [ ] Build internal image generation module
- [ ] Use local stable diffusion models only
- [ ] Define prompt + workflow schema in Rust
- [ ] Implement graph-based image pipelines
- [ ] Add in-memory job manager with status polling
- [ ] Stream results via SSE to frontend
- [ ] Display:
  - [ ] Prompt inputs and generation history
  - [ ] Real-time token + inference stats
  - [ ] Image previews with metadata
- [ ] Link generated images to message/thread context
- [ ] Plan extensibility for style transfer, upscaling, and batch generation

---

## 18. Survival Mode (Books + AI)

- [ ] Index survival content
  - [ ] Water
  - [ ] Food
  - [ ] Shelter
  - [ ] First aid
- [ ] Tag & classify content
- [ ] Enable offline-only interaction
- [ ] Disable internet LLM access when offline mode is active

---

## 19. Optional / Bonus Features

- [ ] Raven companions (Ogg & Vorbis 🪶)
- [ ] Local smart home API (EcoFlow, HomeKit, HA)
- [ ] Whisper (speech-to-text)
- [ ] Coqui TTS (text-to-speech)
- [ ] Plugin system for extending AI agents

---

## 20. Milestones

- [ ] Milestone 1: Copilot Chat Compatibility
- [ ] Milestone 2: Project scaffold + workspace
- [ ] Milestone 3: Auth & DB
- [ ] Milestone 4: Frontend MVP
- [ ] Milestone 5: Local LLM Integration
- [ ] Milestone 6: Book Indexing
- [ ] Milestone 7: Threaded Conversation Explorer
- [ ] Milestone 8: Survival Mode
- [ ] Milestone 9: Image + Voice + Summaries
