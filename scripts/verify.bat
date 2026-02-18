@echo off
color 0F
echo ==================================================
echo      ARK COMPILER :: SYSTEM VERIFICATION
echo ==================================================
echo.
echo [1/2] Checking Python Environment...
python --version
if %ERRORLEVEL% NEQ 0 (
    color 4F
    echo [ERROR] Python not found!
    echo.
    pause
    exit /b 1
)

echo.

echo [2/3] Running THE GAUNTLET (Ark Regression Suite)...
echo.
python meta/gauntlet.py
set GAUNTLET_EXIT=%ERRORLEVEL%
if %GAUNTLET_EXIT% NEQ 0 (
    echo [WARNING] THE GAUNTLET REPORTED FAILURES. CONTINUING TO UNIT TESTS...
)

echo.
echo [3/3] Running Python Unit Tests (Security & Agents)...
echo.
python -m unittest discover tests
if %ERRORLEVEL% NEQ 0 (
    color 4F
    echo.
    echo [ERROR] PYTHON UNIT TESTS FAILED.
    echo.
    pause
    exit /b 1
)
echo.

color 2F
echo.
echo ==================================================
echo [SUCCESS] PYTHON UNIT TESTS PASSED.
echo ==================================================
echo.

if %GAUNTLET_EXIT% NEQ 0 (
    color 4F
    echo.
    echo ==================================================
    echo [WARNING] ARK REGRESSION SUITE FAILED (See above).
    echo ==================================================
    echo.
    echo However, Python Unit Tests PASSED.
    echo.
    pause
    exit /b 1
)

echo.
echo ==================================================
echo [SUCCESS] SYSTEM VERIFIED. ALL SYSTEMS GREEN.
echo ==================================================
echo.
echo You may now post "Victory".
echo.
pause
exit /b 0
