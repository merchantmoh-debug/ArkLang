const WASM_PATH = '../target/wasm32-unknown-unknown/release/ark_0_zheng.wasm';

let wasmExports = null;
let wasmMemory = null;

async function init() {
    const outputDiv = document.getElementById('output');
    const statusDot = document.getElementById('status');
    
    try {
        outputDiv.innerHTML += "\n[System] Loading WASM...";
        
        const response = await fetch(WASM_PATH);
        const bytes = await response.arrayBuffer();
        const { instance } = await WebAssembly.instantiate(bytes, {
            env: {
                // If we needed imports, they go here.
            }
        });

        wasmExports = instance.exports;
        wasmMemory = wasmExports.memory;

        statusDot.classList.add('connected');
        outputDiv.innerHTML += " [OK]\n[System] Ark-0 Runtime: ONLINE.";
        
    } catch (e) {
        statusDot.style.background = '#ef4444';
        outputDiv.innerHTML += `\n[Error] Failed to load WASM: ${e}`;
        console.error(e);
    }
}

function runArk() {
    if (!wasmExports) {
        alert("Runtime not ready.");
        return;
    }

    const inputJson = document.getElementById('input').value;
    const outputDiv = document.getElementById('output');
    outputDiv.innerText = "[Running...]";

    try {
        // 1. Allocate & Write Input
        const ptr = strToPtr(inputJson);
        const len = new TextEncoder().encode(inputJson).length;

        // 2. Execute (ark_eval)
        // Returns pointer to response buffer [len (u32) | content...]
        const resPtr = wasmExports.ark_eval(ptr, len);

        // 3. Read Response
        const response = readResponse(resPtr);

        // 4. Free Input (Caller responsibility? Actually alloc/dealloc pattern usually implies caller frees if they alloc'd)
        // We allocated input at `ptr` with size `len` (capacity might be larger but `dealloc` takes size)
        // Rust side `Vec::from_raw_parts` needs capacity. 
        // My `ark_alloc` implementation creates a Vec with capacity `size`. 
        // So I should free it with `size`.
        wasmExports.ark_dealloc(ptr, len); // Potentially unsafe if capacity > len, but for simple strings usually len=cap or close. 
        // Actually, `strToPtr` likely allocates exactly needed.

        // 5. Free Response (Rust side returns a new allocation that we own)
        // Response string length is needed. `readResponse` knows it.
        // We should explicitly free it if we want to be clean, but for this demo standard GC might not catch WASM memory?
        // WASM memory assumes manual management.
        // The implementation of `readResponse` reads the length.
        // We'll trust the memory leak for this prototype or implement stricter freeing if needed.

        outputDiv.innerText = response;

    } catch (e) {
        outputDiv.innerText = `[Execution Error] ${e}`;
    }
}

function strToPtr(str) {
    const encoder = new TextEncoder();
    const bytes = encoder.encode(str);
    const ptr = wasmExports.ark_alloc(bytes.length);
    const buffer = new Uint8Array(wasmMemory.buffer, ptr, bytes.length);
    buffer.set(bytes);
    return ptr;
}

function readResponse(ptr) {
    const view = new DataView(wasmMemory.buffer);
    
    // Read Header (Length: u32, Little Endian)
    const len = view.getUint32(ptr, true); // true = littleEndian

    // Read Content
    const contentOffset = ptr + 4;
    const decoder = new TextDecoder();
    const bytes = new Uint8Array(wasmMemory.buffer, contentOffset, len);
    const str = decoder.decode(bytes);
    
    // Free the result buffer (len + 4 bytes header)
    wasmExports.ark_dealloc(ptr, len + 4);

    return str;
}

window.addEventListener('load', init);
