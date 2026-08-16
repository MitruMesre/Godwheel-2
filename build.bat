@echo off
setlocal
cd /d "%~dp0"

echo ============================================
echo Godwheel full build
echo ============================================

for %%T in (wasm-pack npm git cargo) do (
    where %%T >nul 2>nul
    if errorlevel 1 (
        echo ERROR: required tool not on PATH: %%T
        goto :fail
    )
)

echo.
echo [1/2] wasm-pack build --dev --target web
wasm-pack build --dev --target web
if errorlevel 1 goto :fail

@REM no ts yet

@REM echo.
@REM echo [2/2] npm run typecheck
@REM rem npm resolves to npm.cmd on Windows - a batch script. Without "call"
@REM rem here, npm.cmd finishing would terminate THIS script too instead of
@REM rem returning control to it (classic Windows batch-calls-batch gotcha).
@REM call npm run typecheck
@REM if errorlevel 1 goto :fail

echo.
echo ============================================
echo BUILD SUCCEEDED
echo ============================================
echo.
pause
exit /b 0

:fail
echo ============================================
echo BUILD FAILED - see error above
echo ============================================
echo.
pause
exit /b 1