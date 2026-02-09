import './components/hero.js';
import './components/terminal.js';
import { loadRuntime } from './runtime.js';

console.log('Ark System Booting...');

document.addEventListener('DOMContentLoaded', async () => {
    // Initialize WASM Runtime
    try {
        await loadRuntime('site/wasm/ark.wasm');
        console.log('Ark WASM Loaded.');
        const term = document.querySelector('ark-terminal');
        if (term) term.log('[System] Kernel Loaded Successfully.');
    } catch (e) {
        console.error('Boot Failed:', e);
        const term = document.querySelector('ark-terminal');
        if (term) term.log(`[System] Boot Error: ${e.message}`);
    }
});
