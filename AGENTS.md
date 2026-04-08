# AGENTS.md

## 1. Purpose

Translat is a desktop translation workstation focused on high-quality, traceable, AI-assisted translation from English to Spanish (Castilian), with emphasis on:

- editorial consistency
- terminology control
- style control
- cost awareness
- human-in-the-loop revision
- reusable parallel corpus knowledge
- reproducible AI context building

This repository must be treated as a product for **document and narrative translation workflows**, not as a generic prompt playground.

---

## 2. Canonical documentation

Before making design or code decisions, agents must treat the following documents as the source of truth, in this order:

1. **PRD**
2. **Technical architecture**
3. **Data model**
4. **Detailed AI action contracts**
5. **Roadmap / backlog**
6. **This AGENTS.md**

If this file conflicts with the updated product docs, the updated product docs win.

---

## 3. Task ID system

### Current canonical task system
Use:

- `TR-1`
- `TR-2`
- `TR-155`
- `TR-2500`

### Legacy IDs
Older tasks such as:

- `A1–A5`
- `B1–B4`
- `C1–C5`
- `D1–D5`

must be treated as **legacy references**, not as the active task ID system.

When creating or updating work items, include explicit mapping such as:

- `Legacy refs`
- `Adjusts`
- `Extends later`
- `Keeps`

Do not destroy historical traceability.

---

## 4. Product model to assume

Agents must reason with the following product model:

### 4.1 Atomic persistence unit
A **segment** is the smallest persisted text unit.

### 4.2 Operational translation unit
A **translation chunk** is the primary AI translation unit for long-form documents.

This means:

- a document is normalized first
- then segmented
- then grouped into **translation chunks**
- chunks may include contextual overlap
- chunks are translated with layered editorial context

### 4.3 Context is layered
Do not assume “project-only context”.

Effective translation context may include:

- project defaults
- master glossary
- work/book glossary
- project glossary
- active style profile
- active rules by action scope
- translation memory
- nearby document context
- chapter/section accumulated context
- chunk-local context

### 4.4 Long documents
Long documents and novels must **not** assume monolithic one-shot translation in a single AI call.

Use:

- chunk-based translation
- batch flows
- persisted jobs
- resumable processing
- QA after translation

---

## 5. Editorial model to assume

### 5.1 Glossary layers
The system supports layered terminology.

Expected scopes include:

- `master`
- `work`
- `project`

Agents must preserve or extend explicit precedence handling, not flatten it casually.

### 5.2 Style profiles
Style is not a cosmetic add-on.

Style profiles may govern:

- tone
- formality
- ritual / solemn register
- descriptive density
- grimdark / epic intensity
- treatment of titles and ranks
- dialogue conventions
- naming policies
- technical vs sacred vocabulary balance

### 5.3 Rules
Rules are operational and may be scoped by action, such as:

- `translation`
- `retranslation`
- `qa`
- `export`
- `consistency_review`

Do not model rules as passive notes only.

---

## 6. AI action model to assume

The current product direction assumes these action patterns:

### Primary operational action
- `translate_chunk`

### Fine-grained actions
- `translate_segment`
- `retranslate_with_comment`

### Document workflow actions
- `translate_document`
- `run_consistency_qa`
- `estimate_cost`

### Knowledge actions
- `align_parallel_corpus`
- `search_specific_translation`

Agents must not keep treating `translate_segment` as the only or main translation path for large documents.

---

## 7. Architecture assumptions

When working on code, agents should preserve the following architectural direction:

### Core document flow
- ingest document
- normalize text
- detect structure
- create sections/chapters
- segment content
- build translation chunks
- compose editorial context
- translate
- run QA
- persist outputs and artifacts
- export

### Important modules
Expected module boundaries now include concepts like:

- document structure
- chunking
- context building
- glossary resolution
- translation memory
- AI orchestration
- QA
- job execution

### Backend concepts
The architecture now expects explicit support for:

- `TranslationChunk`
- `DocumentSection`
- `ChapterContext`
- glossary layer resolution
- rules by action scope
- persisted jobs
- `qa_findings`

---

## 8. Data model assumptions

Agents must align code with the updated data model direction.

Important entities include:

- `Project`
- `Document`
- `DocumentSection`
- `Segment`
- `TranslationChunk`
- `TranslationChunkSegment`
- `ChapterContext`
- `Glossary`
- `GlossaryEntry`
- `StyleProfile`
- `RuleSet`
- `Rule`
- `TranslationMemoryEntry`
- `TaskRun`
- `JobQueue`
- `QaFinding`

### Important persistence principles
- segment = atomic persistence unit
- chunk = operational translation unit
- editorial context must be reproducible
- job progress must be resumable
- QA findings must be persistable and traceable

---

## 9. Agent behaviour expectations

### 9.1 Prefer documentation-first changes
At the current phase of Translat, prefer:

1. PRD alignment
2. architecture alignment
3. data model alignment
4. AI contracts alignment
5. AGENTS / skills alignment
6. implementation

Do not jump into implementation when foundational docs are out of sync.

### 9.2 Respect traceability
When proposing or implementing changes:

- reference relevant `TR-X` tasks
- preserve legacy task mappings
- avoid ambiguous design drift
- keep names and behaviour consistent across docs and code

### 9.3 Do not silently collapse concepts
Do not collapse these concepts into one another:

- segment vs chunk
- project glossary vs master glossary
- style vs rules
- interactive action vs job execution
- QA checks vs translation output
- document section vs document segment

### 9.4 Human-in-the-loop is mandatory
Translat is not a fully autonomous translator.

Design decisions must preserve:

- reviewability
- editability
- auditability
- cost visibility
- controlled reuse of prior translations

---

## 10. Implementation guidance

### When editing code
Agents should ask:

- Is this change aligned with the PRD?
- Does it preserve chunk-based translation?
- Does it preserve layered editorial context?
- Does it preserve action-scoped rules?
- Does it preserve reproducible task runs and QA artifacts?

### When editing docs
Agents should update documents consistently across:

- PRD
- architecture
- data model
- AI contracts
- backlog / roadmap
- AGENTS / skills

Do not update just one document if the concept spans the system.

### UI / UX workflow policy
Use Figma or Figma MCP as a required design step for **structural UI work**, including:

- new product workflows
- new primary screens or workspaces
- multi-panel layout changes
- operational state design
- reusable component families
- UI decisions that directly shape upcoming implementation tasks

Figma is **not required** for minor UI work, such as:

- small bug fixes
- copy-only changes
- spacing/alignment tweaks
- minor responsive adjustments
- technical refactors without meaningful UX impact

Every UI-related task should explicitly state one of:

- `Figma required`
- `Figma not required`

When Figma is not required, include a short justification.

---

## 11. Current priority

At the current stage, Translat priority is:

- align all core docs to the new operating model
- make `AGENTS.md` and skills consume that updated model
- only then continue implementation work using the revised docs as canonical guidance

This means agents should currently prefer **structural alignment and coherence** over opportunistic feature coding.

---

## 12. Practical instruction for coding agents

When working on Translat, assume:

- the repo is evolving toward **chunk-based editorial translation**
- editorial context is layered and resolved, not hardcoded
- long-form translation is batch/job-friendly and resumable
- QA is part of the product model, not an afterthought
- documentation is currently being upgraded and must stay internally consistent
