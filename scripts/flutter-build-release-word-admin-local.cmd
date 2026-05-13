@echo off
setlocal
set "REPO_ROOT=%~dp0.."
set "ENV_FILE=%REPO_ROOT%\.env.word-admin.local"

if not exist "%ENV_FILE%" (
  echo Missing "%ENV_FILE%".
  echo Create it with WORD_ADMIN_API_URL=http://^<computer-lan-ip^>:8787
  exit /b 1
)

for /f "usebackq tokens=1,* delims==" %%A in ("%ENV_FILE%") do (
  if /i "%%A"=="WORD_ADMIN_API_URL" set "WORD_ADMIN_API_URL=%%B"
)

if "%WORD_ADMIN_API_URL%"=="" (
  echo WORD_ADMIN_API_URL is missing in "%ENV_FILE%".
  exit /b 1
)

pushd "%REPO_ROOT%\apps\flutter_mobile"
flutter build apk --release --dart-define=CLOUD_BACKEND=word_admin --dart-define=WORD_ADMIN_API_URL=%WORD_ADMIN_API_URL% %*
set "EXIT_CODE=%ERRORLEVEL%"
popd
exit /b %EXIT_CODE%
