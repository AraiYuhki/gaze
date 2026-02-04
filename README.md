# gaze

A lightweight Git TUI tool that delegates all operations to the git CLI.

gaze provides a simple terminal interface for everyday Git tasks:
- **Status View**: Stage, unstage, and discard changes
- **Tree View**: Browse repository files with lazy-loading
- **Log View**: Explore commit history with graph visualization

## Philosophy

- **CLI delegation**: All Git operations go through the `git` command
- **Minimal footprint**: No libgit2, no gitoxide, just process calls
- **Explicit exclusions**: submodule and LFS are intentionally unsupported
