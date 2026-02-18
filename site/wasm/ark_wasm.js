/**
 * Ark WASM Loader
 * Loads and instantiates the Ark WASM module in the browser.
 */
const ArkWasm = {
  /**
   * Load an Ark WASM binary from a URL path.
   * @param {string} path - Relative or absolute path to the .wasm file.
   * @returns {Promise<WebAssembly.WebAssemblyInstantiatedSource>}
   */
  async load(path) {
    const importObject = {
      env: {
        // Stub imports for functions the WASM module expects from the host
        ark_print: (ptr, len) => {
          // In a real integration, this would read UTF-8 from WASM memory
          console.log(`[ark_print] ptr=${ptr} len=${len}`);
        },
        ark_log: (ptr, len) => {
          console.log(`[ark_log] ptr=${ptr} len=${len}`);
        },
        ark_time_now: () => Math.floor(Date.now() / 1000),
        ark_random: () => Math.random(),
        // Memory import (if needed)
        memory: new WebAssembly.Memory({ initial: 16, maximum: 256 }),
      },
      wasi_snapshot_preview1: {
        // Minimal WASI stubs for compatibility
        fd_write: () => 0,
        fd_read: () => 0,
        fd_close: () => 0,
        fd_seek: () => 0,
        proc_exit: (code) => { console.log(`[proc_exit] code=${code}`); },
        environ_get: () => 0,
        environ_sizes_get: () => 0,
        clock_time_get: () => 0,
        args_get: () => 0,
        args_sizes_get: () => 0,
        random_get: (ptr, len) => {
          const view = new Uint8Array(ArkWasm._memory.buffer, ptr, len);
          crypto.getRandomValues(view);
          return 0;
        },
      },
    };

    try {
      // Try streaming compilation first (faster)
      if (typeof WebAssembly.instantiateStreaming === 'function') {
        const result = await WebAssembly.instantiateStreaming(fetch(path), importObject);
        if (result.instance.exports.memory) {
          ArkWasm._memory = result.instance.exports.memory;
        }
        return result;
      }
    } catch (e) {
      console.warn('Streaming compilation failed, falling back to ArrayBuffer:', e.message);
    }

    // Fallback: fetch as ArrayBuffer
    const response = await fetch(path);
    const bytes = await response.arrayBuffer();
    const result = await WebAssembly.instantiate(bytes, importObject);
    if (result.instance.exports.memory) {
      ArkWasm._memory = result.instance.exports.memory;
    }
    return result;
  },

  _memory: null,
};

// Make available globally for test harness
if (typeof window !== 'undefined') {
  window.ArkWasm = ArkWasm;
}
if (typeof module !== 'undefined') {
  module.exports = ArkWasm;
}
