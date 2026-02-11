const WASM_PATH = 'wasm/ark.wasm';

let wasmExports = null;
let wasmMemory = null;

async function init() {
    const outputDiv = document.getElementById('output');
    const statusDot = document.getElementById('wasm-status');

    outputDiv.innerText = "[SYSTEM] Boot Sequence Initiated...\n";

    try {
        const response = await fetch(WASM_PATH);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const bytes = await response.arrayBuffer();
        const { instance } = await WebAssembly.instantiate(bytes, { env: {} });

        wasmExports = instance.exports;
        wasmMemory = wasmExports.memory;

        statusDot.classList.add('active');
        logSystem("WASM Module Loaded.");
        logSystem("Ark-0 Runtime: ONLINE.");
        logSystem("Virtual Nervous System: ACTIVE.");

    } catch (e) {
        statusDot.classList.add('error');
        logError(`Failed to load WASM: ${e}`);
    }
}

function logSystem(msg) {
    const out = document.getElementById('output');
    out.innerText += `[SYS] ${msg}\n`;
    out.scrollTop = out.scrollHeight;
}

function logError(msg) {
    const out = document.getElementById('output');
    out.innerText += `[ERR] ${msg}\n`;
    out.scrollTop = out.scrollHeight;
}

function runArk() {
    if (!wasmExports) {
        logError("Runtime not ready.");
        return;
    }

    const inputJson = document.getElementById('input').value;
    const outputDiv = document.getElementById('output');

    outputDiv.innerText += `\n> EXECUTE_VECTOR [LEN: ${inputJson.length}]\n`;

    try {
        const ptr = strToPtr(inputJson);
        const len = new TextEncoder().encode(inputJson).length;

        const resPtr = wasmExports.ark_eval(ptr, len);
        const response = readResponse(resPtr);

        outputDiv.innerText += `${response}\n`;
        outputDiv.innerText += `[OIS] Transaction Complete.\n`;

    } catch (e) {
        logError(`Execution Fault: ${e}`);
    }
    outputDiv.scrollTop = outputDiv.scrollHeight;
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
    const len = view.getUint32(ptr, true);
    const contentOffset = ptr + 4;
    const decoder = new TextDecoder();
    const bytes = new Uint8Array(wasmMemory.buffer, contentOffset, len);
    const str = decoder.decode(bytes);
    wasmExports.ark_dealloc(ptr, len + 4);
    return str;
}

// Ctrl+Enter to Run
document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        runArk();
    }
});

window.addEventListener('load', init);
