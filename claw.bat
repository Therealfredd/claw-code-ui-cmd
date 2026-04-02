@echo off
setlocal

rem Windows does not set HOME; many Unix-style tools expect it.
if not defined HOME set "HOME=%USERPROFILE%"

set "PROJECT_DIR=%~dp0"
set "RUST_DIR=%PROJECT_DIR%rust"
set "RELEASE_EXE=%RUST_DIR%\target\release\claw.exe"
set "DEBUG_EXE=%RUST_DIR%\target\debug\claw.exe"

rem Prefer release build, fall back to debug
if exist "%RELEASE_EXE%" set "CLAW_EXE=%RELEASE_EXE%"
if exist "%DEBUG_EXE%" if not defined CLAW_EXE set "CLAW_EXE=%DEBUG_EXE%"

rem If no binary found, build it first
if not defined CLAW_EXE goto :build
goto :ollama_start

:build
echo No claw binary found. Building now (this may take a minute)...
echo.
pushd "%RUST_DIR%"
cargo build --release
if errorlevel 1 (
    echo.
    echo Build failed. Check the output above for errors.
    popd
    pause
    exit /b 1
)
popd
set "CLAW_EXE=%RELEASE_EXE%"

:ollama_start
rem Check if Ollama is already running
set "OLLAMA_STARTED=0"
where ollama.exe >nul 2>&1
if errorlevel 1 goto :launch

tasklist /fi "imagename eq ollama.exe" 2>nul | find /i "ollama.exe" >nul 2>&1
if not errorlevel 1 goto :launch

rem Ollama not running - start it in the background
echo Starting Ollama...
start /b "" ollama serve >nul 2>&1
set "OLLAMA_STARTED=1"

rem Give Ollama a moment to initialise
ping -n 3 127.0.0.1 >nul 2>&1

:launch
rem Try Windows Terminal first (full colour support).
rem Fall back to running directly in this CMD window.
where wt.exe >nul 2>&1
if not errorlevel 1 (
    if "%OLLAMA_STARTED%"=="1" (
        wt.exe -d "%CD%" cmd /k ""%CLAW_EXE%" & taskkill /f /im ollama.exe >nul 2>&1"
    ) else (
        wt.exe -d "%CD%" cmd /k "%CLAW_EXE%"
    )
    goto :eof
)

"%CLAW_EXE%"
if errorlevel 1 (
    echo.
    echo Claw exited with an error.
)

rem Stop Ollama if we started it
if "%OLLAMA_STARTED%"=="1" (
    echo Stopping Ollama...
    taskkill /f /im ollama.exe >nul 2>&1
)

:eof
endlocal
