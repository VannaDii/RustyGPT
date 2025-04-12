# ✅ RustyGPT — Full Implementation Plan (TODO Checklist)

> A privacy-respecting, full-stack AI platform in Rust (Axum + Yew) that integrates local models, PostgreSQL, OAuth (Apple/GitHub), SSE streaming, stored procedures, and AI tools for survival, conversation, and reasoning.

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
- [ ] Add `Makefile` or `justfile`
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
  - [ ] LLM handle
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
  - [ ] Short-lived JWT
  - [ ] Secure refresh cookie

---

## 🧠 4. AI Model Integration (LLM)

- [ ] Add local model backends
  - [ ] `ollama`
  - [ ] `open-webui`
  - [ ] `comfyui`
- [ ] Define trait abstraction for engine
- [ ] Stream completions over SSE
- [ ] Cache completions and embeddings

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
  - [ ] `ollama`
  - [ ] `open-webui`
  - [ ] `comfyui`
- [ ] Mount volumes
  - [ ] `/array/data/books`
  - [ ] `/array/data/media`
- [ ] Enable GPU pass-through

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
  - [ ] Model download
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
  - [ ] Devil's Advocate
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
- [ ] Offer "offline survival mode"

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

## 🖼️ 15. ComfyUI Image Generation

- [ ] Send job to ComfyUI
- [ ] Monitor job via socket or REST
- [ ] Stream updates via SSE
- [ ] Display:
  - [ ] Prompt UI
  - [ ] Image history
- [ ] Link generated images to messages

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

## 📑 17. Moderation & Guardrails

- [ ] Add local-only moderation engine
- [ ] Detect unsafe prompts
- [ ] Rewrite or clarify problematic inputs

---

## 🧩 18. Optional / Bonus Features

- [ ] Raven companions (Ogg & Vorbis 🪶)
- [ ] Local smart home API (EcoFlow, HomeKit, HA)
- [ ] Whisper (speech-to-text)
- [ ] Coqui TTS (text-to-speech)
- [ ] Plugin system for extending AI agents

---

## ✅ 19. Milestones

- [ ] Milestone 1: Project scaffold + workspace
- [ ] Milestone 2: Auth & DB
- [ ] Milestone 3: Frontend MVP
- [ ] Milestone 4: Local LLM Integration
- [ ] Milestone 5: Book Indexing
- [ ] Milestone 6: Threaded Conversation Explorer
- [ ] Milestone 7: Survival Mode
- [ ] Milestone 8: Image + Voice + Summaries
