@echo off
title MCP Command Center (headless)
cd /d "%~dp0"
if not exist "target\release\mcp-command-center.exe" (
  echo [build] compilation release...
  cargo build --release -p mcp-command-center
)
target\release\mcp-command-center.exe
