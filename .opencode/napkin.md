# Napkin Runbook

## Curation Rules
- Re-prioritize on every read.
- Keep recurring, high-value notes only.
- Max 10 items per category.
- Each item includes date + "Do instead".

## Execution & Validation (Highest Priority)
1. **[2026-03-17] Feature plans use "_plan" suffix**
   Do instead: create `docs/<feature>_plan.md` for each new feature, mark completed items as user validates.

## Shell & Command Reliability
1. **[2026-03-17] Use `workdir` param instead of `cd` in bash**
   Do instead: always use `workdir` parameter for bash commands to avoid persistent directory state issues.

## User Directives
1. **[2026-03-17] Keep responses under 4 lines**
   Do instead: answer concisely without preamble/postamble unless user asks for detail.

2. **[2026-03-17] Changelog at project root**
   Do instead: maintain CHANGELOG.md with semantic versioning and Keep a Changelog format.
