export async function loadRuntime(wasmPath) {
    // 1. Define Imports (Hook Rust externs to JS)
    const imports = {
        env: {
            ark_print: (ptr, len) => {
                // Read string from memory
                const memory = new Uint8Array(window.wasm_instance.exports.memory.buffer);
                const bytes = memory.subarray(ptr, ptr + len);
                const str = new TextDecoder("utf8").decode(bytes);
                // Log to terminal if available
                const term = document.querySelector('ark-terminal');
                if (term) term.log(str);
            },
            ark_ask_ai: (ptr, len) => {
                 const term = document.querySelector('ark-terminal');
                 if (term) term.log("[Neuro-Bridge] Thinking locally (Simulation)...");
                 return 0; // Placeholder ptr
            }
        }
    };

    // 2. Fetch & Instantiate
    try {
        const response = await fetch(wasmPath);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const bytes = await response.arrayBuffer();
        const results = await WebAssembly.instantiate(bytes, imports);
        
        // 3. Export to Window for debug
        window.wasm_instance = results.instance;
        
        // 4. Bind Eval
        window.ark_eval = (code) => {
            const instance = window.wasm_instance;
            const { ark_alloc, ark_dealloc, ark_eval, memory } = instance.exports;

            // 1. Encode Input
            const encoder = new TextEncoder();
            const bytes = encoder.encode(code);
            const len = bytes.length;

            // 2. Allocate Memory
            const ptr = ark_alloc(len);
            
            // 3. Write to Memory
            const memBuf = new Uint8Array(memory.buffer);
            memBuf.set(bytes, ptr);

            // 4. Call WASM
            const resPtr = ark_eval(ptr, len);

            // 5. Read Response
            // Response format: [len (4 bytes LE)] [content...]
            const view = new DataView(memory.buffer);
            const resLen = view.getUint32(resPtr, true); // Little Endian
            
            const resContentPtr = resPtr + 4;
            const resBytes = new Uint8Array(memory.buffer, resContentPtr, resLen);
            const decoder = new TextDecoder("utf8");
            const result = decoder.decode(resBytes);

            // 6. Cleanup (Dealloc input and output)
            // Note: ark_eval deallocs input? No, we own it.
            // Actually implementation of ark_eval doesn't dealloc input.
            // But we should dealloc it here? 
            // The Rust side `Vec::from_raw_parts` in `ark_dealloc` handles it.
            // Let's verify ownership. `ark_alloc` returns a forgotten Vec.
            // We should dealloc input.
            ark_dealloc(ptr, len);

            // We must also dealloc the response buffer (len + 4 bytes)
            ark_dealloc(resPtr, resLen + 4);

            return result;
        };
        
    } catch (e) {
        console.warn("WASM Load Failed (Mocking enabled):", e);
    }
}
