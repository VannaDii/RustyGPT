# ✅ RustyGPT — Full Implementation Plan (TODO Checklist)

A privacy-respecting, full-stack AI platform in Rust (Axum + Yew) that integrates local models, PostgreSQL, OAuth (Apple/GitHub), SSE streaming, stored procedures, and AI tools for survival, conversation, and reasoning.

**NOW PRIORITIZING:** Full GitHub Copilot Chat compatibility via OpenAI-compatible `/v1/chat/completions` and `/v1/models` endpoints.

**All LLM inference, model loading, pipeline orchestration, and task execution is handled locally—no external API calls or model services.**

---

## 🔊 0. Copilot Chat-Compatible API First

- [ ] Scaffold `rustygpt-api` crate to serve Copilot-compatible endpoints
  - [x] `POST /v1/chat/completions` with OpenAI schema
  - [ ] `GET /v1/models` listing available model(s)
- [ ] Static response MVP
  - [ ] Return dummy assistant message
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
  - [x] Unit + integration tests for `/v1/chat/completions`
  - [ ] Manual validation in GitHub Copilot Chat

---

## 📦 1. Project Structure & Workspace

- [ ] Set up `Cargo.toml` workspace with crates
  - [ ] `rustygpt-common`
  - [ ] `rustygpt-api`
  - [ ] `rustygpt-frontend`
  - [ ] `rustygpt-model`
  - [ ] `rustygpt-db`
  - [ ] `rustygpt-index`
  - [ ] `rustygpt-utils`
- [ ] Add Makefile or justfile
- [ ] Add `.cargo/config.toml` for targets

---

## 🚀 2. Backend (Axum)

- [ ] Setup Axum API project
- [ ] Add OpenAPI support via `utoipa` or `paperclip`
- [ ] Define route tree
  - [ ] `/auth`
  - [ ] `/chat`
  - [ ] `/search`
  - [ ] `/admin`
- [ ] Define `AppState`
  - [ ] Config
  - [ ] DB pool
  - [ ] Cache
  - [ ] Model runtime handle
- [ ] Add middleware
  - [ ] Tracing
  - [ ] Auth (JWT / Cookie)
  - [ ] CORS
  - [ ] Compression
- [ ] Implement SSE streaming endpoint

---

## 🛂 3. Authentication & Accounts

- [ ] Local login
  - [ ] Email + password (argon2)
- [ ] External auth
  - [ ] Apple Sign In
  - [ ] GitHub OAuth
- [ ] Stored procedures in Postgres
  - [ ] Create user
  - [ ] Login
  - [ ] Validate session
  - [ ] Link OAuth identity
- [ ] Token handling
  - [ ] Short- [ ]lived JWT
  - [ ] Secure refresh cookie

---

## 🧠 4. AI Model Integration (LLM)

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

## 📚 5. File, Book, and Knowledge Indexing

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

## 🧪 6. Testing & Coverage

- [ ] Add `cargo llvm-cov` to workflow
  - [ ] Command: `cargo llvm-cov --workspace --html --output-dir .coverage && open .coverage/index.html`
- [ ] Write unit tests
  - [ ] API routes
  - [ ] SSE stream
  - [ ] Stored procedures
- [ ] Add Yew tests for frontend logic
- [ ] Target 90%+ coverage in all crates

---

## 🖥️ 7. Frontend (Yew)

- [ ] Set up Yew project
- [ ] Integrate TailwindCSS + Trunk
- [ ] Implement pages
  - [ ] Auth (Sign In, Sign Up)
  - [ ] Chat
  - [ ] Search
  - [ ] Admin
- [ ] Build components
  - [ ] ChatBox
  - [ ] Streaming tokens
  - [ ] Threaded view
- [ ] Use shared types via `rustygpt-common`

---

## 📦 8. Dockerized Environment

- [ ] Define Docker Compose stack
  - [ ] `rustygpt-api`
  - [ ] `rustygpt-frontend`
  - [ ] `postgres`
  - [ ] `rustygpt-model` (internal LLM server)
- [ ] Mount volumes
  - [ ] `/array/data/books`
  - [ ] `/array/data/media`
- [ ] Enable GPU pass-through (REQUIRED For Linux and macOS)

---

## 🧰 9. Tooling & Dev Experience

- [ ] Add `COPILOT_INSTRUCTIONS.md`
  - [ ] Idiomatic Rust enforcement
  - [ ] Explicit type links in docstrings
  - [ ] No unnecessary clones
  - [ ] Crate version synchronization
- [ ] Add `dev.md`
  - [ ] Common commands
- [ ] Add CLI tools or scripts
  - [ ] DB seed
  - [ ] Model download/init
  - [ ] Lint / check / coverage

---

## 💬 10. Conversation Explorer

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

## 🧠 11. Critical Thinking Tools

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

## 🛡️ 12. Privacy & Offline Use

- [ ] Run all inference locally
- [ ] No cloud dependencies
- [ ] Store all data locally
- [ ] Offer “offline survival mode”

---

## 🔒 13. Roles & Permissions

- [ ] Define roles
  - [ ] Admin
  - [ ] User
  - [ ] Read-only
- [ ] Protect endpoints with role-based guards
- [ ] Enforce message access control

---

## 🧠 14. Prompt Enrichment & Memory

- [ ] Generate thread summaries
- [ ] Token-bounded memory injection
- [ ] Enrich prompts with:
  - [ ] Glossary
  - [ ] System instructions
  - [ ] Relevant prior messages

---

## 🖼️ 15. RustyGPT Image Generation

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

## 🏕️ 16. Survival Mode (Books + AI)

- [ ] Index survival content
  - [ ] Water
  - [ ] Food
  - [ ] Shelter
  - [ ] First aid
- [ ] Tag & classify content
- [ ] Enable offline-only interaction
- [ ] Disable internet LLM access when offline mode is active

---

## 🧩 17. Optional / Bonus Features

- [ ] Raven companions (Ogg & Vorbis 🪶)
- [ ] Local smart home API (EcoFlow, HomeKit, HA)
- [ ] Whisper (speech-to-text)
- [ ] Coqui TTS (text-to-speech)
- [ ] Plugin system for extending AI agents

---

## ✅ 18. Milestones

- [ ] Milestone 1: Copilot Chat Compatibility
- [ ] Milestone 2: Project scaffold + workspace
- [ ] Milestone 3: Auth & DB
- [ ] Milestone 4: Frontend MVP
- [ ] Milestone 5: Local LLM Integration
- [ ] Milestone 6: Book Indexing
- [ ] Milestone 7: Threaded Conversation Explorer
- [ ] Milestone 8: Survival Mode
- [ ] Milestone 9: Image + Voice + Summaries
