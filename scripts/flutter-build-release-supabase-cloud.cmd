@echo off
setlocal
set "REPO_ROOT=%~dp0.."
set "ENV_FILE=%REPO_ROOT%\.env.supabase.local"
set "FLUTTER_ROOT=D:\flutter\flutter"
set "ANDROID_HOME=D:\Android\Sdk"
set "ANDROID_SDK_ROOT=%ANDROID_HOME%"
set "JAVA_HOME=C:\Program Files\Microsoft\jdk-21.0.10.7-hotspot"
set "GRADLE_USER_HOME=%REPO_ROOT%\apps\mobile\.gradle-home"
set "GRADLE_OPTS=-Xmx1536m -XX:MaxMetaspaceSize=512m -XX:ReservedCodeCacheSize=128m -Dfile.encoding=UTF-8 -Dorg.gradle.daemon=false -Dorg.gradle.workers.max=1"
set "PATH=%JAVA_HOME%\bin;%FLUTTER_ROOT%\bin;%FLUTTER_ROOT%\bin\cache\dart-sdk\bin;%ANDROID_HOME%\platform-tools;%PATH%"

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
call "%FLUTTER_ROOT%\bin\flutter.bat" pub get
if errorlevel 1 (
  popd
  exit /b 1
)
call "%FLUTTER_ROOT%\bin\flutter.bat" build apk --release --dart-define=SUPABASE_URL=%SUPABASE_URL% --dart-define=SUPABASE_ANON_KEY=%SUPABASE_ANON_KEY% %*
set "EXIT_CODE=%ERRORLEVEL%"
popd
exit /b %EXIT_CODE%
