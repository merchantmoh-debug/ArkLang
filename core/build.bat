@echo off
REM Ark Cargo Build Wrapper
REM Sets up MinGW GCC for the cc crate used by wasmtime's build script.
set PATH=C:\msys64\mingw64\bin;%PATH%
set CC=C:\msys64\mingw64\bin\gcc.exe
set AR=C:\msys64\mingw64\bin\ar.exe
cargo %*
