@echo off
setlocal
set "REPO_ROOT=%~dp0.."
set "SUPABASE=%REPO_ROOT%\.tools\supabase-cli\node_modules\.bin\supabase.cmd"

if not exist "%SUPABASE%" (
  echo Supabase CLI not found at "%SUPABASE%".
  echo Install it with:
  echo npm.cmd install --prefix "%REPO_ROOT%\.tools\supabase-cli" --cache "%REPO_ROOT%\.npm-cache" supabase@2.95.6
  exit /b 1
)

"%SUPABASE%" %*
