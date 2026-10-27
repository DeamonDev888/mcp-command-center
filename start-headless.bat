@echo off
title MCP Control (headless)
cd /d "%~dp0"
if not exist "target\release\mcp-control.exe" (
  echo [build] compilation release...
  cargo build --release -p mcp-control
)
target\release\mcp-control.exe
