---
Review the pull request diff against the full repository context. For every function added, modified, or deleted, trace all direct and indirect callers and callees across files. Analyze how the change alters dataflow, state, control flow, and error handling along those call chains.

CRITICAL: Do NOT report a defect unless you can cite at least one concrete caller (file + line) in the repository that exercises the flawed code path with the violating input. If no such caller exists in the repo, the finding is invalid. Output only defects that meet this standard.

Identify real defects the change introduces—broken contracts, unsafe data propagation, resource mismanagement, unreachable code, or invariant violations. For each defect, output a concise description and the exact file paths with line numbers that evidence the issue, referencing both the changed code and the traced caller/callee lines. Do not note style, formatting, or performance opinions. If no defect is detected, produce no output. Ground every statement strictly in the diff and repository evidence.
---
