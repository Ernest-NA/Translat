---
name: pr-review
description: Review pull request diffs for Translat and keep findings focused on actionable bugs, regressions, security issues, scope creep, and missing validation. Use when Codex is asked to review a PR manually or when maintaining the automated PR review workflow, prompt, or schema.
---

# pr-review

## Purpose
Use this skill when reviewing a pull request diff for Translat, either manually in Codex or through the automated PR review workflow.

## When to use
- reviewing a pull request for bugs, regressions, security issues, or missing tests
- maintaining the automated PR review prompt or workflow
- checking whether a diff stays within the intended task scope
- validating that frontend, backend, and persistence wiring remain coherent across a PR

## Expected outcomes
- findings stay focused on actionable issues introduced by the PR
- comments cite exact files and lines
- severity is clear and findings are ordered from most important to least important
- summaries stay brief and only follow the findings
- review outputs stay concise and low-verbosity

## Working rules
- default to a review-first mindset
- review the PR diff, not the codebase in the abstract
- prioritize correctness, regressions, security, maintainability, and missing tests
- ignore stylistic nits unless they block understanding or hide a real defect
- call out scope creep when a PR quietly mixes unrelated work
- if there are no findings, say that explicitly and mention residual risks briefly
- if asked to remediate findings, use a bounded loop with at most 2 fix iterations
- make minimal, targeted changes only
- run the smallest relevant validation checks
- stop once the PR is acceptable for human review
- never enter an indefinite self-review/self-fix loop

## Recommended checklist
1. Read `AGENTS.md`, `docs/architecture/system-overview.md`, and `docs/product/git-workflow-and-releases.md` before finalizing findings.
2. Inspect only the files changed by the PR unless surrounding code is needed for correctness.
3. Verify that file paths and line references point to the changed side of the diff.
4. Check whether tests or validation cover the introduced behavior.
