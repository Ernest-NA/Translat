# prompt-strategy-by-action

## Purpose
Use this skill when working on prompt construction, prompt catalogs, context compaction, and output contracts for Translat AI actions.

## When to use
- adding or changing prompt templates
- changing prompt layering rules
- implementing prompt builders
- reviewing context size and relevance
- aligning prompts with output schemas and action contracts

## Expected outcomes
- prompts stay versioned and action-specific
- output structure remains stable and machine-validatable
- context remains minimal but sufficient
- glossary, style, and rule priority stay explicit

## Working rules
- do not place prompts directly in random controllers or UI code
- separate stable prompt instructions from runtime variables
- keep formatting and output expectations unambiguous
- avoid overloading one prompt with multiple unrelated jobs

## Recommended checklist
1. Identify the exact action affected.
2. Preserve the layer order: role, task policy, output contract, project context, terminology, style/rules, local context, user comment.
3. Keep user comments constrained so they do not break the response contract.
4. Update prompt strategy documentation if the action behavior changes materially.
