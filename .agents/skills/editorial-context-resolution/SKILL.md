# editorial-context-resolution

## Purpose
Use this skill when implementing or refining how Translat resolves the editorial context used by AI actions.

Editorial context in Translat is layered, explicit, and reproducible. This skill exists to prevent context assembly from devolving into ad hoc project defaults or undocumented heuristics.

## Canonical references
Before changing anything, align with:
1. PRD
2. Technical architecture
3. Data model
4. Detailed AI action contracts
5. `AGENTS.md`

## When to use
- implementing glossary layer resolution
- determining active style profile and active rule sets
- building context for `translate_chunk`, `translate_segment`, `translate_document`, or QA actions
- changing precedence between master/work/project editorial layers
- implementing chapter/section accumulated context
- refining how translation memory, corpus evidence, and local context are selected

## Expected outcomes
- editorial context remains reproducible
- glossary layers stay explicit and ordered
- style and rules remain separate concerns
- accumulated context remains bounded and relevant
- action-specific context differences stay visible in code and docs

## Working rules
- do not rely on project defaults alone when richer editorial context exists
- do not merge glossary, style, and rules into one opaque blob
- preserve precedence and traceability
- keep context minimal but sufficient
- prefer deterministic selection over vague heuristics
- document any change that affects reproducibility or action behavior

## Recommended checklist
1. Identify the target action and its `action_scope`.
2. Resolve active glossary layers in precedence order.
3. Resolve the active style profile.
4. Resolve active rules for the action scope.
5. Select relevant translation memory and corpus support.
6. Add chapter/section accumulated context when relevant.
7. Add local chunk or segment context.
8. Verify the final editorial context is bounded, inspectable, and reproducible.

## Red flags
- “project context” used as an undefined catch-all
- glossary precedence implied but not persisted
- style instructions mixed with hard business rules
- chapter context added without size controls
- different actions reusing the same context blindly
