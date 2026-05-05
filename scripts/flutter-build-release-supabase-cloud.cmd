@echo off
setlocal
set "REPO_ROOT=%~dp0.."
set "ENV_FILE=%REPO_ROOT%\.env.supabase.local"

if not exist "%ENV_FILE%" (
  echo Missing "%ENV_FILE%".
  echo Create it with SUPABASE_URL and SUPABASE_ANON_KEY entries.
  exit /b 1
)

for /f "usebackq tokens=1,* delims==" %%A in ("%ENV_FILE%") do (
  if /i "%%A"=="SUPABASE_URL" set "SUPABASE_URL=%%B"
  if /i "%%A"=="SUPABASE_ANON_KEY" set "SUPABASE_ANON_KEY=%%B"
)

if "%SUPABASE_URL%"=="" (
  echo SUPABASE_URL is missing in "%ENV_FILE%".
  exit /b 1
)

if "%SUPABASE_ANON_KEY%"=="" (
  echo SUPABASE_ANON_KEY is missing in "%ENV_FILE%".
  exit /b 1
)

pushd "%REPO_ROOT%\apps\flutter_mobile"
flutter build apk --release --dart-define=SUPABASE_URL=%SUPABASE_URL% --dart-define=SUPABASE_ANON_KEY=%SUPABASE_ANON_KEY% %*
set "EXIT_CODE=%ERRORLEVEL%"
popd
exit /b %EXIT_CODE%
