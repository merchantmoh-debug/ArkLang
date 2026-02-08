$env:CC_wasm32_unknown_unknown = "c:\Users\Stran\.gemini\antigravity\scratch\tools\llvm-mingw-20251216-ucrt-x86_64\bin\clang.exe"
$env:AR_wasm32_unknown_unknown = "c:\Users\Stran\.gemini\antigravity\scratch\tools\llvm-mingw-20251216-ucrt-x86_64\bin\llvm-ar.exe"
$env:CFLAGS_wasm32_unknown_unknown = "-target wasm32-unknown-unknown"

cargo build --target wasm32-unknown-unknown --release --verbose > build.log 2>&1
