# parallel-corpus-alignment

## Purpose
Use this skill when implementing or refining the EN/ES parallel corpus subsystem of Translat.

## When to use
- creating or updating corpus ingestion flows
- implementing segment alignment logic
- refining align_parallel_corpus behavior
- implementing search_specific_translation support
- promoting corpus findings into glossary or translation memory

## Expected outcomes
- the corpus subsystem remains separate from generic translation flows
- alignment stays inspectable and confidence-based
- literal and non-literal equivalences are handled explicitly
- promoted findings remain traceable to corpus evidence

## Working rules
- do not treat the corpus subsystem as a simple text search feature
- preserve alignment confidence and ambiguity handling
- keep query results structured and reusable
- ensure corpus results can be promoted without bypassing traceability

## Recommended checklist
1. Clarify whether the work targets ingestion, alignment, search, or promotion.
2. Preserve source/target segment references and confidence signals.
3. Keep results structured for UI inspection and later reuse.
4. Update product or architecture docs if corpus behavior changes materially.
