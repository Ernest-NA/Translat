# parallel-corpus-alignment

## Purpose
Use this skill when implementing or refining the EN/ES parallel corpus subsystem of Translat.

This skill applies to corpus ingestion, alignment, structured search, confidence-aware reuse, and promotion of corpus evidence into glossary layers or translation memory.

## When to use
- creating or updating corpus ingestion flows
- implementing or refining alignment logic
- improving `align_parallel_corpus`
- implementing or refining `search_specific_translation`
- promoting corpus findings into glossary entries or translation memory
- exposing corpus evidence to support chunk-level translation or QA
- refining how corpus evidence is stored, scored, reviewed, or reused

## Product assumptions
Agents using this skill must assume:

- the corpus subsystem is a structured evidence source, not a generic text search feature
- corpus evidence may support **segment-level** or **chunk-level** translation workflows
- promoted findings must preserve provenance and confidence
- glossary promotion must respect glossary scope and editorial precedence
- translation memory reuse must remain auditable and human-reviewable

## Expected outcomes
- the corpus subsystem remains separate from generic translation flows
- alignment stays inspectable and confidence-based
- literal and non-literal equivalences are handled explicitly
- promoted findings remain traceable to corpus evidence
- corpus outputs remain reusable by translation, QA, and editorial review workflows

## Working rules
- do not treat the corpus subsystem as a simple keyword search feature
- preserve alignment confidence and ambiguity handling
- keep query results structured and reusable
- preserve source/target references and enough surrounding context for inspection
- ensure corpus results can be promoted without bypassing editorial traceability
- do not auto-promote corpus findings into master/work/project glossary layers without an explicit policy or review step
- do not conflate corpus evidence with validated translation memory

## Editorial integration rules
Corpus findings may be promoted into:

- **glossary layers** (`master`, `work`, `project`) when they represent terminological knowledge
- **translation memory** when they represent reusable bilingual translation evidence

When promoting findings, preserve at least:
- provenance
- confidence
- source/target text references
- scope or target layer
- reviewer decision, if applicable

## Reuse expectations
Corpus outputs should be usable by:
- `translate_chunk`
- `translate_segment`
- `run_consistency_qa`
- glossary suggestion flows
- translation memory suggestion flows

Support for chunk-aware reuse is important when the corpus helps disambiguate terminology or repeated phrasing across longer passages.

## Recommended checklist
1. Clarify whether the work targets ingestion, alignment, search, review, or promotion.
2. Preserve source/target segment references, confidence signals, and provenance.
3. Keep results structured for UI inspection and later reuse.
4. Decide whether the result is corpus evidence, glossary candidate, or translation memory candidate.
5. Ensure promotion respects glossary scope, precedence, and traceability.
6. Update product or architecture docs if corpus behavior changes materially.
