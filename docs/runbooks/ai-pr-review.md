# AI PR review

## Objective

Describe how Translat runs automated AI-assisted pull request reviews after a PR is opened or updated, with an emphasis on high-signal, breadth-first issue detection on the first pass while keeping API cost controlled.

## Current workflow

- Workflow file: `.github/workflows/ai-pr-review.yml`
- Triggered on non-draft pull requests targeting `develop` or `main`
- Runs after `opened`, `reopened`, `synchronize`, and `ready_for_review`
- Uses the repository skill `.agents/skills/pr-review/SKILL.md` as the review rubric
- Publishes or updates a single sticky top-level PR comment instead of creating a new comment on every push

## Review behavior

- Focuses on actionable issues introduced by the diff
- Prioritizes correctness, regressions, security, maintainability, API and contract changes, and missing tests
- Uses a breadth-first first pass to maximize recall before ranking findings by severity
- Internally enumerates more candidate issues than it will publish, then deduplicates by root cause and returns the strongest independent findings
- Returns up to 5 findings for small PRs, up to 8 findings for medium PRs, and up to 10 findings for large or high-risk PRs
- Prefers breadth across files and defect categories on the first pass rather than over-concentrating on only the top 1 to 3 severe issues
- Keeps automated review output concise and low-verbosity
- Uses findings only for concrete, high-confidence defects that are actionable for the author
- Reports lower-confidence concerns as residual risks or open questions instead of promoting them to findings
- Excludes generated files, lockfiles, snapshots, formatter-only churn, and other non-semantic changes from meaningful diff calculations whenever possible
- Skips pull requests whose meaningful diff is entirely docs-only or lockfile-only
- Leaves findings, residual risks, and open questions in a single PR comment
- The AI review is advisory only; it does not approve or block merges, and human review remains required

## Required configuration

- GitHub Actions secret: `OPENAI_API_KEY`
- Optional repository variable: `CODEX_MODEL_MINI`
  - Default: the repository-defined mini review model
  - Falls back to legacy `CODEX_MODEL` if `CODEX_MODEL_MINI` is unset
- Bot-authored pull requests are allowed to trigger the review flow, so Dependabot PRs are covered too

## Model selection policy

- The mini review model is the default and only automatic review model for pull request review
- The workflow does not escalate automatically to any larger model
- Cost control and broad review coverage should be achieved through prompt quality, meaningful diff filtering, and publication strategy rather than model escalation
- Diff-size stats are still recorded in the workflow summary for operator visibility
- Diff-size and review thresholds should be computed from the meaningful diff after excluding generated files, lockfiles, snapshots, and formatter-only changes

## Review pass strategy

- The review should behave as a two-step process, even if implemented in a single call:
  - Candidate generation: identify as many independent actionable issues as possible
  - Publication: deduplicate by root cause and publish the strongest set within the configured cap
- If the reviewer hits the publication cap, it should prefer broad coverage across files and defect categories
- The reviewer should avoid publishing multiple findings that are only symptoms of the same underlying defect
- Missing or inadequate tests should be raised as a finding only when the diff shows a concrete behavior change, regression risk, or unverified edge case
- Cosmetic observations, style nits, or low-value comments should be omitted unless they hide a real maintainability or correctness problem

## Meaningful diff policy

- Meaningful diff calculations should ignore files or hunks that do not materially affect runtime behavior or review complexity
- The following should be excluded whenever possible:
  - Lockfiles
  - Generated code
  - Build artifacts
  - Snapshot files
  - Formatter-only rewrites
  - Pure import sorting changes
  - Pure whitespace-only changes
  - Large vendored or machine-produced outputs
- If a pull request mixes meaningful code changes with excluded noise, the excluded noise should not drive review thresholds or reduce review quality
- If a pull request is entirely docs-only or lockfile-only after meaningful diff filtering, the workflow should skip the AI review and record the reason in the workflow summary

## Prompt and schema files

- Prompt template: `.github/codex/pr-review-prompt.md`
- Structured output schema: `.github/codex/pr-review-schema.json`

## Notes

- If `OPENAI_API_KEY` is missing, the workflow skips cleanly and records the reason in the workflow summary
- The workflow posts a top-level PR comment rather than inline review comments to keep the rollout simpler and easier to maintain
- The review is prioritized, but it should aim for high recall on the first pass rather than reporting only the top few issues
- Later runs may still expose additional issues because the diff changes after fixes, but the first pass should aim to surface the broadest useful set of independent findings
- For remediation work driven by AI review findings, prefer a review-first pass, keep fixes minimal, use at most 2 fix iterations, and run only the smallest relevant validation checks before handing back to human review
- The workflow summary records the selected model source, the reason it was chosen, and the meaningful diff-size stats used by the policy
- If the review quality or noise level needs adjustment, update the skill first, then the prompt or workflow only when necessary
- Pull request title and body are passed into the prompt through environment variables to avoid shell-escaping issues with quotes or multiline text
- If the reviewer reaches the configured finding cap, the comment should make it clear that additional residual concerns may still exist