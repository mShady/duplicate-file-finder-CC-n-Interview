# Code Review Guidelines

## Always check

- New functions/methods have appropriate error handling
- API changes are backward-compatible or clearly documented as breaking
- No hardcoded secrets, tokens, or credentials
- Tests cover the happy path and at least one edge case for new logic
- New endpoints have authentication/authorization checks

## Security (always flag)

- Hardcoded secrets or credentials → CRITICAL
- SQL/command injection risks → CRITICAL
- Missing input validation on user-facing endpoints → HIGH
- Sensitive data in logs or error messages → HIGH

## Style

- Follow existing patterns in the codebase
- Prefer descriptive variable names over abbreviations
- Keep functions focused — flag functions doing too many things

## Skip

- Formatting-only changes
- Auto-generated files
- Dependency lock file changes (\*.lock)
- Comments and documentation-only changes (unless inaccurate)
