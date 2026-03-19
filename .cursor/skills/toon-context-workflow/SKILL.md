---
name: toon-context-workflow
description: Manage and troubleshoot the toon-context MCP server. Use when toon-context is disconnected, tools fail, the user asks about TOON setup, or when restarting/diagnosing the semantic search backend.
---

# toon-context Workflow

## Server Details

- **Config**: Project `.cursor/mcp.json` or `~/.cursor/mcp.json`
- **Wrapper**: `run_mcp.sh` in repo root (uses `TOON_BACKEND` env)
- **MCP registration**: `~/.cursor/projects/<workspace>/mcps/toon-context/`

## Quick Diagnosis

1. Check if tools are registered (path varies by workspace):

```bash
ls ~/.cursor/projects/*/mcps/toon-context/tools/
```

If any directory exists with `.json` files, the server is connected.

2. If missing, try calling any toon tool — a "server does not exist" error confirms it's down.

3. Check if the process is running:

```bash
ps aux | grep mcp_server.py | grep -v grep
```

## Restart Procedure

User must:

1. Open **Cursor Settings** → **MCP**
2. Find **toon-context** and click the restart button

## Tool Selection Rules

| toon-context status | Search tool      | Read tool          |
|---------------------|------------------|--------------------|
| Connected           | `toon_search`    | `toon_read_file`   |
| Disconnected        | Grep / Glob      | Read               |

## Available Tools (when connected)

- **toon_read_file**: Read a single file with function metadata in TOON format
- **toon_read_files**: Batch read multiple files in TOON
- **toon_search**: Semantic code search via natural language query, results in TOON
- **toon_compress**: Convert arbitrary JSON to TOON (40% token savings)

## Common Failures

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| "MCP server does not exist" | Server not started by Cursor | User restarts from MCP settings |
| Tool call hangs | Server process crashed | User restarts from MCP settings |
| Empty search results | Index not built | Run indexing from backend directory |
| Import errors in hook | venv packages missing or TOON_BACKEND not set | Set TOON_BACKEND; `pip install toon-format` in backend venv |
