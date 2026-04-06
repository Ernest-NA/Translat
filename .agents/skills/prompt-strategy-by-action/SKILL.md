# prompt-strategy-by-action

## Purpose
Use this skill when working on prompt construction, prompt catalogs, context compaction, and output contracts for Translat AI actions.

This skill assumes the current Translat model:
- prompts are action-specific and versioned
- `translate_chunk` is the primary translation action for long-form documents
- editorial context is layered and resolved explicitly
- chunk-local and chapter/section context matter for literary and document translation
- prompts must support machine-validatable outputs and traceable side effects

## Canonical references
Before changing prompts, align with:
1. PRD
2. Technical architecture
3. Data model
4. Detailed AI action contracts
5. `AGENTS.md`

## When to use
- adding or changing prompt templates
- changing prompt layering rules
- implementing prompt builders
- reviewing context size and relevance
- aligning prompts with output schemas and action contracts
- introducing or changing `translate_chunk`, `translate_segment`, `translate_document`, `retranslate_with_comment`, or QA prompts

## Expected outcomes
- prompts stay versioned and action-specific
- output structure remains stable and machine-validatable
- context remains minimal but sufficient
- glossary, style, and rule priority stay explicit
- chunk-level prompts preserve enough local and accumulated context without becoming unbounded

## Working rules
- do not place prompts directly in random controllers or UI code
- separate stable prompt instructions from runtime variables
- keep formatting and output expectations unambiguous
- avoid overloading one prompt with multiple unrelated jobs
- preserve the distinction between glossary layers, style profiles, and action-scoped rules
- do not inject encyclopedia-sized lore blindly; only pass the minimum relevant context
- prefer reproducible context assembly over ad hoc “smart” stuffing

## Layering model
Use a deliberate layer order. The recommended order is:

1. role / system policy
2. action policy
3. output contract
4. editorial context policy
5. active glossary layers
6. active style profile
7. active rules for the current action scope
8. translation memory / corpus evidence
9. chapter or section accumulated context
10. local chunk or segment context
11. user comment or revision instruction

Do not collapse these layers into a single vague “project context”.

## Action-specific expectations
### `translate_chunk`
Must typically include:
- target language and translation objective
- active glossary layers in precedence order
- style profile
- rules for `translation`
- nearby context or overlap context
- accumulated chapter/section context when relevant
- exact output contract for all segments inside the chunk

### `translate_segment`
Use when the operation is intentionally narrow:
- local correction
- focused rework
- isolated segment support
- micro review flows

Do not treat it as the default strategy for long-form documents.

### `translate_document`
Should be designed as a job/batch orchestration layer built from chunk prompts, not as a single monolithic translation prompt for long documents.

### `retranslate_with_comment`
Must constrain the user comment so it cannot break:
- output schema
- required terminology
- active rules
- traceability assumptions

### `run_consistency_qa`
Must be able to inspect:
- terminology consistency
- style consistency
- chunk-to-chunk continuity
- unresolved violations against glossary/rules/style

## Recommended checklist
1. Identify the exact action affected.
2. Preserve prompt versioning.
3. Confirm whether the prompt targets chunk, segment, document job, QA, or retraduction.
4. Preserve glossary layer precedence explicitly: master -> work -> project, unless the active configuration says otherwise.
5. Keep style and rule layers separate.
6. Keep user comments constrained so they do not break the response contract.
7. Ensure prompt size stays bounded and context compaction remains intentional.
8. Update prompt strategy documentation if the action behavior changes materially.

## Red flags
- prompt text embedded in UI or controller code
- one giant prompt trying to solve translation, QA, review, and corpus search at once
- local chunk context passed without glossary or active rules
- project defaults treated as the only editorial source
- style instructions mixed ambiguously with mandatory rule constraints
