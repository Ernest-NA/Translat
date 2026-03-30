# System Overview

## Objective
Provide a desktop application for orchestrating English-to-Spanish translation workflows with AI, including project management, terminology, style rules, parallel corpora, cost tracking, and supervised refinement.

## Context
- Business domain: AI-assisted translation and editorial workflow.
- Primary users: translators, reviewers, editors, and advanced hobbyists.
- Core flows: project creation, document import, segmentation, translation, retraduction, historical review, parallel corpus search, and QA.
- External integrations: OpenAI API, GitHub, and Notion.

## Architecture principles
- Keep the design modular and evolvable.
- Prefer explicit action contracts over ad hoc LLM calls.
- Preserve strong traceability for changes, costs, and outputs.
- Document decisions that affect multiple modules.

## Suggested sections to complete
- Logical components
- Data flow
- External services
- Security constraints
- Deployment model
