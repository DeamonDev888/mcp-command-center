@echo off
title MCP Command Center
cd /d "%~dp0"
if not exist "target\release\mcp-command-center.exe" (
  echo [build] compilation release...
  cargo build --release
)
echo [UI] http://127.0.0.1:5090
target\release\mcp-command-center.exe
pause
