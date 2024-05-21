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

:: Compress the dll
where upx.exe
if %ERRORLEVEL% == 0 (
    echo Found upx
    upx --best --lzma target/release/balalib.dll
) else (
    echo upx not found
    echo Skipping compression
)

:: copy the dll to %appdata%/Balatro
copy target\release\balalib.dll "%appdata%\Balatro\balalib.dll"