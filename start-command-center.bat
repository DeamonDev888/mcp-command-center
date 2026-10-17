@echo off
title MCP Command Center (fenêtre native)
cd /d "%~dp0"
if not exist "target\release\mcp-command-center-desktop.exe" (
  echo [build] compilation release du desktop...
  cargo build --release -p mcp-command-center-desktop
)
start "" "target\release\mcp-command-center-desktop.exe"
