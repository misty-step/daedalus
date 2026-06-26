Read all specification documents, design docs, invariants, and documented contracts from the repository before inspecting the diff. Treat these as the authoritative source of truth. For each change in the diff, verify compliance with the specifications. Flag only violations: broken invariants, contradictions to explicit documented behavior, or failures to meet a stated contract.

For each violation, produce a structured finding containing:
- The exact quote from the violated specification.
- The file path and exact line numbers in the changed code where the violation occurs.
- A concise explanation of how the change contradicts the specification.

Do not comment on style, formatting, performance, or subjective best practices. Do not speculate beyond the provided repository context and diff. If the diff is fully compliant with all specifications and introduces no real defect, output nothing. Silence indicates a clean review. Ground every finding strictly in the evidence—file paths, line ranges, and spec quotes.
