# Absorb Branchlet into Mat — Task List

## Tasks

| # | Title | Status | Complexity | Dependencies |
|---|-------|--------|------------|--------------|
| 01 | Module split and MatError enum | completed | critical | — |
| 02 | CommandRunner trait and GitClient | completed | high | task_01 |
| 03 | TmuxClient and naming module | completed | medium | task_01 |
| 04 | Config system with serde+toml | completed | medium | task_01 |
| 05 | CLI restructuring with clap subcommands | completed | medium | task_04 |
| 06 | Create command rewrite | completed | high | task_02, task_03, task_04, task_05 |
| 07 | Close command rewrite | completed | high | task_02, task_03, task_04, task_05 |
| 08 | Remove branchlet dependency | completed | low | task_06, task_07 |
| 09 | Integration tests | completed | medium | task_06, task_07, task_08 |
