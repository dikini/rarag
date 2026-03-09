# Prompt Contract Minimal Example

Use this as a compact scaffold for agent-facing tasks.

```text
<context>
Project: <project-name>
Task: Analyze repository behavior with bounded retrieval evidence.
</context>

<instruction_priority>
1. System constraints
2. Developer constraints
3. Task-specific contracts in this prompt
</instruction_priority>

<constraints>
- Use bounded retrieval (`limit`, optional `symbol_path`, explicit history selectors).
- Distinguish direct evidence from inference.
- Do not claim completion without verification output.
</constraints>

<output_contract>
- Return: summary, files changed, verification status.
- Do not return unrelated analysis.
</output_contract>

<evidence_contract>
- Required classes: code and document.
- Optional class: history (only with explicit selectors).
- Prefer specs/ops for present behavior over plans/changelog.
</evidence_contract>

<verification_contract>
- Run: scripts/doc-lint.sh --changed --strict-new
- Run: scripts/run-shell-tests.sh --changed
- If checks fail: fix and rerun before final response.
</verification_contract>
```

Plain-language fallback for compatibility:

- Treat tagged blocks as readability helpers only.
- Repeat critical constraints in normal prose for model robustness (including GPT-5.3 behavior).
