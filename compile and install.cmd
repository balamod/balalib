@echo off

:: check if the game is installed
if exist "%appdata%\Balatro" (
    echo Found balatro save folder
) else (
    echo Balatro save folder not found
    echo Please run the game at least once before running this script
    exit /b 1
)

:: check if cargo is installed
where cargo.exe
if %ERRORLEVEL% == 0 (
    echo Found cargo
) else (
    echo Cargo not found
    echo Please install Rust and Cargo from https://www.rust-lang.org/tools/install
    exit /b 1
)

:: building the dll
cargo build --release

:: copy the dll to %appdata%/Balatro
copy target\release\balalib.dll "%appdata%\Balatro\balalib.dll"