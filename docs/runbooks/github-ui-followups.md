# GitHub UI follow-ups for B3.1

## Objective
List the repository guardrails that are intentionally outside B3 because they must be configured manually by the repository owner in GitHub UI.

## B3.1 manual actions
1. Disable CodeQL default setup if it is still enabled, so the advanced workflow in `.github/workflows/codeql.yml` is the only active CodeQL configuration.
2. Configure branch protection or rulesets for `develop` and `main`.
3. Choose the required status checks after the workflow names and job names from B3 are accepted as stable.
4. Review code scanning alert notification settings and owner-facing security preferences.
5. Review Dependabot alert and security update preferences in GitHub UI.

## Exact failure signal for item 1
- If a CodeQL workflow run uploads SARIF successfully and then fails with `CodeQL analyses from advanced configurations cannot be processed when the default setup is enabled`, GitHub default setup is still active.
- That conflict must be resolved in GitHub UI. It is not fixed by changing the repository workflow alone.

## Why this file exists
These controls are critical, but they are not versionable in the repository. Keeping them listed here prevents accidental scope creep in B3 while making the remaining owner work explicit and auditable.
