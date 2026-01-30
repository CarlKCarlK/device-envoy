@echo off
REM ==========================================
REM  Gather top-level, src/, and examples/ .rs files into target\gathers\all_code.txt
REM ==========================================

set OUTPUT=target\gathers\all_code.txt
if not exist target\gathers mkdir target\gathers
del "%OUTPUT%" 2>nul

echo Gathering Rust files from ., src, and examples...
echo ========================================== > "%OUTPUT%"

REM --- Top-level .rs files ---
for %%F in (*.rs) do (
    echo ==== FILE: %%F ==== >> "%OUTPUT%"
    type "%%F" >> "%OUTPUT%"
    echo. >> "%OUTPUT%"
)

REM --- src/ directory ---
if exist src (
    for /r src %%F in (*.rs) do (
        echo ==== FILE: %%F ==== >> "%OUTPUT%"
        type "%%F" >> "%OUTPUT%"
        echo. >> "%OUTPUT%"
    )
)

REM --- examples/ directory ---
if exist examples (
    for /r examples %%F in (*.rs) do (
        echo ==== FILE: %%F ==== >> "%OUTPUT%"
        type "%%F" >> "%OUTPUT%"
        echo. >> "%OUTPUT%"
    )
)

echo ========================================== >> "%OUTPUT%"
echo Done! Combined files written to %OUTPUT%
pause
