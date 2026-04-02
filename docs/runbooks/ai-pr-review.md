# AI PR review

## Objective

Describe how Translat runs automated AI-assisted pull request reviews after a PR is opened or updated.

## Current workflow

- Workflow file: `.github/workflows/ai-pr-review.yml`
- Triggered on non-draft pull requests targeting `develop` or `main`
- Runs after `opened`, `reopened`, `synchronize`, and `ready_for_review`
- Uses the repository skill `.agents/skills/pr-review/SKILL.md` as the review rubric
- Publishes or updates a sticky PR comment instead of creating a new comment on every push

## Review behavior

- Focuses on actionable issues introduced by the diff
- Prioritizes correctness, regressions, security, maintainability, and missing tests
- Skips docs-only and lockfile-only pull requests
- Leaves findings, residual risks, and open questions in a single PR comment
- Does not block merge by itself; human review remains required

## Required configuration

- GitHub Actions secret: `OPENAI_API_KEY`
- Optional repository variable: `CODEX_MODEL`
  - Default: `gpt-5.4`
- Bot-authored pull requests are allowed to trigger the review flow, so Dependabot PRs are covered too.

## Prompt and schema files

- Prompt template: `.github/codex/pr-review-prompt.md`
- Structured output schema: `.github/codex/pr-review-schema.json`

## Notes

- If `OPENAI_API_KEY` is missing, the workflow skips cleanly and records the reason in the workflow summary.
- The workflow posts a top-level PR comment rather than inline review comments to keep the first rollout simpler and easier to maintain.
- If the review quality or noise level needs adjustment, update the skill first, then the prompt or workflow only when necessary.
- Pull request title and body are passed into the prompt through environment variables to avoid shell-escaping issues with quotes or multiline text.
