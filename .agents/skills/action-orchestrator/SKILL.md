# action-orchestrator

## Purpose
Use this skill when implementing or changing the typed AI action system of Translat.

## When to use
- adding a new AI action
- changing ActionRequest, ActionEstimate, or ActionResult flows
- updating ActionRegistry, ActionHandler, ContextBuilder, or OutputValidator
- modifying task_run persistence tied to AI execution
- reviewing action-level traceability or error handling

## Expected outcomes
- AI work remains routed through typed actions
- context assembly stays explicit and reusable
- outputs are validated before mutating state
- task runs remain the traceability backbone

## Working rules
- do not introduce ad hoc calls to the model outside the orchestrator flow
- separate estimation, execution, validation, and persistence concerns
- preserve action versioning and prompt version traceability
- keep batch and interactive modes explicit

## Recommended checklist
1. Confirm whether the change belongs to action definition, handler, context, validation, or persistence.
2. Ensure the action contract stays typed and versioned.
3. Keep side effects explicit and auditable.
4. Update the corresponding documentation if the action contract or flow changes.
