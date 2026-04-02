# AI PR review

## Objective

Describe how Translat runs automated AI-assisted pull request reviews after a PR is opened or updated.

## Current workflow

- Workflow file: `.github/workflows/ai-pr-review.yml`
- Triggered on non-draft pull requests targeting `develop` or `main`
- Runs after `opened`, `reopened`, `synchronize`, and `ready_for_review`
- Uses the repository skill `.agents/skills/pr-review/SKILL.md` as the review rubric
- Publishes or updates a single sticky top-level PR comment instead of creating a new comment on every push

## Review behavior

- Focuses on actionable issues introduced by the diff
- Prioritizes correctness, regressions, security, maintainability, and missing tests
- Returns up to 5 findings per run, ordered by severity
- Uses `gpt-5.4-mini` by default for routine PR reviews to control API cost
- Escalates automatically to `gpt-5.4` for pull requests targeting `main` and for large diffs
- Skips pull requests whose diff is entirely docs-only or lockfile-only
- Leaves findings, residual risks, and open questions in a single PR comment
- The AI review is advisory only; it does not approve or block merges, and human review remains required

## Required configuration

- GitHub Actions secret: `OPENAI_API_KEY`
- Optional repository variable: `CODEX_MODEL_MINI`
  - Default: `gpt-5.4-mini`
- Optional repository variable: `CODEX_MODEL_FULL`
  - Default: `gpt-5.4`
- Bot-authored pull requests are allowed to trigger the review flow, so Dependabot PRs are covered too.

## Model selection policy

- `gpt-5.4-mini` is the default review model for normal PRs into `develop`
- `gpt-5.4` is selected automatically when the PR targets `main`
- `gpt-5.4` is also selected automatically for large diffs
- Current large-diff thresholds:
  - `>= 25` changed files, or
  - `>= 1200` changed lines (`additions + deletions`)

## Prompt and schema files

- Prompt template: `.github/codex/pr-review-prompt.md`
- Structured output schema: `.github/codex/pr-review-schema.json`

## Notes

- If `OPENAI_API_KEY` is missing, the workflow skips cleanly and records the reason in the workflow summary.
- The workflow posts a top-level PR comment rather than inline review comments to keep the first rollout simpler and easier to maintain.
- The review is prioritized, not exhaustive. If a PR contains several defects, later runs may expose additional issues after the first set is fixed because the diff has changed.
- Large-diff escalation should ignore lockfiles, generated files, snapshots, and formatter-only artifacts when feasible. This is a recommended refinement; the current workflow does not enforce those exclusions in the size thresholds.
- The workflow summary records the selected model, the reason it was chosen, and the diff-size stats used by the policy.
- If the review quality or noise level needs adjustment, update the skill first, then the prompt or workflow only when necessary.
- Pull request title and body are passed into the prompt through environment variables to avoid shell-escaping issues with quotes or multiline text.
