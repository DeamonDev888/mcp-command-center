@echo off
title MCP Control (fenêtre native)
cd /d "%~dp0"
if not exist "target\release\mcp-control-desktop.exe" (
  echo [build] compilation release du desktop...
  cargo build --release -p mcp-control-desktop
)
if not exist "target\release\mcp-control-desktop.exe" (
  echo [erreur] build impossible ou cargo absent — lance "cargo build --release -p mcp-control-desktop" manuellement.
  pause
  exit /b 1
)
start "" "target\release\mcp-control-desktop.exe"
