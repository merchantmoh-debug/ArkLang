import {EditorView, basicSetup} from "codemirror"
import {keymap} from "@codemirror/view"
import {Compartment} from "@codemirror/state"
import {defaultKeymap, indentWithTab} from "@codemirror/commands"
import {StreamLanguage} from "@codemirror/language"
import {oneDark} from "@codemirror/theme-one-dark"

// --- WASM Module ---
let wasmModule = null;
let wasmReady = false;

async function initWasm() {
    try {
        // Try to load the WASM module (built with wasm-pack)
        const wasm = await import('../core/pkg/ark_0_zheng.js');
        await wasm.default(); // Initialize WASM
        wasmModule = wasm;
        wasmReady = true;
        logToConsole("WASM Runtime loaded — client-side execution enabled.", "success");
    } catch (e) {
        console.warn("WASM module not available, falling back to server-side execution:", e);
        logToConsole("WASM not available — using server-side execution.", "dim");
    }
}

// --- Ark Language Definition ---
const arkKeywords = new Set([
    "let", "mut", "func", "if", "else", "while", "for", "return",
    "import", "struct", "match", "true", "false", "nil", "async", "await"
]);

const arkTypes = new Set([
    "Int", "Float", "String", "Bool", "List", "Dict", "Any", "void"
]);

const arkLanguage = StreamLanguage.define({
    token(stream) {
        if (stream.eatSpace()) return null;

        // Comments
        if (stream.match("//")) {
            stream.skipToEnd();
            return "comment";
        }

        // Strings
        if (stream.match('"')) {
            while (!stream.eol()) {
                if (stream.next() == '"' && stream.string[stream.pos-2] != '\\') break;
            }
            return "string";
        }

        // Numbers
        if (stream.match(/^\d+(\.\d+)?/)) return "number";

        // Keywords & Identifiers
        if (stream.match(/^[a-zA-Z_][a-zA-Z0-9_]*/)) {
            const word = stream.current();
            if (arkKeywords.has(word)) return "keyword";
            if (arkTypes.has(word)) return "typeName";
            if (stream.peek() === '(') return "functionName";
            return "variableName";
        }

        // Operators
        if (stream.match(/^[\+\-\*\/=<>!&|]+/)) return "operator";

        // Brackets
        if (stream.match(/^[\{\}\(\)\[\]]/)) return "punctuation";

        stream.next();
        return null;
    }
});

// --- Examples ---
const EXAMPLES = {
    "Hello World": `// Hello World in Ark
print("Hello, Ark Sovereign World!")`,
    "Factorial": `// Recursive factorial
func factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

result := factorial(10)
print(result)`,
    "Fibonacci": `// Fibonacci sequence
func fib(n) {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}

i := 0
while i < 10 {
    print(fib(i))
    i += 1
}`,
    "Pipe Operator": `// Functional pipe operator
func double(x) { return x * 2 }
func add_one(x) { return x + 1 }

result := 5 |> double |> add_one
print(result)`,
    "Match Expression": `// Pattern matching
value := 42
match value {
    1 => print("one"),
    42 => print("the answer"),
    _ => print("something else")
}`
};

// --- Editor Setup ---
let view;
let themeCompartment = new Compartment();
let currentFontSize = 14;

function initEditor() {
    // Check for shared code in URL
    const urlParams = new URLSearchParams(window.location.search);
    const sharedCode = urlParams.get('code');
    let startCode = sharedCode ? atob(sharedCode) : EXAMPLES["Hello World"];

    view = new EditorView({
        doc: startCode,
        extensions: [
            basicSetup,
            keymap.of([indentWithTab, ...defaultKeymap]),
            arkLanguage,
            themeCompartment.of(oneDark),
            EditorView.updateListener.of((update) => {
                if (update.selectionSet) {
                    updateCursorPos(update.state);
                }
            })
        ],
        parent: document.getElementById("editor")
    });

    updateCursorPos(view.state);
}

function updateCursorPos(state) {
    const pos = state.selection.main.head;
    const line = state.doc.lineAt(pos);
    const col = pos - line.from + 1;
    document.getElementById("cursor-pos").textContent = `Ln ${line.number}, Col ${col}`;
}

// --- Execution Logic ---
async function runCode() {
    const code = view.state.doc.toString();

    logToConsole("Running...", "info", true);
    const startTime = performance.now();

    if (wasmReady) {
        // Client-side WASM execution
        try {
            const resultJson = wasmModule.ark_eval_source(code);
            const result = JSON.parse(resultJson);
            const elapsed = (performance.now() - startTime).toFixed(1);

            if (result.stdout) {
                logToConsole(result.stdout);
            }

            if (result.error) {
                logToConsole(result.error, "error");
            } else if (result.result !== null && result.result !== undefined) {
                logToConsole(`→ ${JSON.stringify(result.result)}`, "dim");
            }

            logToConsole(`Completed in ${elapsed}ms (WASM)`, "dim");

        } catch (e) {
            logToConsole(`WASM execution failed: ${e.message}`, "error");
        }
    } else {
        // Server-side fallback
        try {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), 10000);

            const response = await fetch('/api/run', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ code: code }),
                signal: controller.signal
            });

            clearTimeout(timeoutId);

            if (!response.ok) {
                throw new Error(`Server Error: ${response.status} ${response.statusText}`);
            }

            const result = await response.json();
            const elapsed = (performance.now() - startTime).toFixed(1);

            if (result.stdout) logToConsole(result.stdout);
            if (result.stderr) logToConsole(result.stderr, "error");
            if (result.error) logToConsole(result.error, "error");

            logToConsole(`Completed in ${elapsed}ms (server)`, "dim");

        } catch (e) {
            if (e.name === 'AbortError') {
                logToConsole("Execution timed out (10s limit).", "error");
            } else {
                logToConsole(`Failed to execute: ${e.message}`, "error");
                logToConsole("Build WASM with: wasm-pack build --target web core/", "dim");
            }
        }
    }
}

function logToConsole(msg, type = "normal", isSystem = false) {
    const outputDiv = document.getElementById("console-output");
    const entry = document.createElement("div");
    entry.className = "log-entry";

    const timestamp = new Date().toLocaleTimeString('en-US', { hour12: false });

    entry.innerHTML = `
        <div class="log-timestamp">[${timestamp}]</div>
        <div class="log-content log-${type}">${escapeHtml(msg)}</div>
    `;

    outputDiv.appendChild(entry);
    outputDiv.scrollTop = outputDiv.scrollHeight;
}

function escapeHtml(unsafe) {
    return unsafe
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;")
         .replace(/\n/g, "<br>");
}

// --- UI Handlers ---
function setupUI() {
    // Examples Sidebar
    const list = document.getElementById("examples-list");
    if (list) {
        Object.keys(EXAMPLES).forEach(name => {
            const item = document.createElement("div");
            item.className = "sidebar-item";
            item.textContent = name;
            item.onclick = () => {
                const currentCode = view.state.doc.toString();
                if (currentCode !== EXAMPLES[name] && currentCode.length > 50) {
                     if(!confirm("Replace current code with example?")) return;
                }

                view.dispatch({
                    changes: {from: 0, to: view.state.doc.length, insert: EXAMPLES[name]}
                });

                document.querySelectorAll(".sidebar-item").forEach(el => el.classList.remove("active"));
                item.classList.add("active");
            };
            list.appendChild(item);
        });
    }

    // Run Button
    document.getElementById("run-btn").addEventListener("click", runCode);

    // Clear Button
    document.getElementById("clear-btn").addEventListener("click", () => {
        document.getElementById("console-output").innerHTML = "";
        logToConsole("Console cleared.", "dim");
    });

    // Share Button
    document.getElementById("share-btn").addEventListener("click", () => {
        const code = view.state.doc.toString();
        const b64 = btoa(code);
        const url = new URL(window.location);
        url.searchParams.set('code', b64);
        window.history.pushState({}, '', url);
        navigator.clipboard.writeText(url.toString()).then(() => {
            logToConsole("Link copied to clipboard!", "success");
        });
    });

    // Theme Toggle
    const themeBtn = document.getElementById("theme-btn");
    themeBtn.addEventListener("click", () => {
        const isDark = !document.body.classList.contains("light-theme");
        if (isDark) {
            document.body.classList.add("light-theme");
            // Switch to default (light) theme by passing empty array
            view.dispatch({effects: themeCompartment.reconfigure([])});
            themeBtn.innerHTML = '<span class="icon">☀</span>';
        } else {
            document.body.classList.remove("light-theme");
            // Switch to One Dark
            view.dispatch({effects: themeCompartment.reconfigure(oneDark)});
            themeBtn.innerHTML = '<span class="icon">◑</span>';
        }
    });

    // Sovereign Mode Toggle
    const sovereignBtn = document.getElementById("sovereign-btn");
    if (sovereignBtn) {
        // Inject styles if not present
        if (!document.getElementById("sovereign-style")) {
            const style = document.createElement("style");
            style.id = "sovereign-style";
            style.textContent = `
                body.sovereign-mode {
                    --bg-color: #000000;
                    --sidebar-bg: #0a0a0a;
                    --card-bg: #0f0f0f;
                    --text-primary: #00ff41;
                    --text-secondary: #008f11;
                    --accent-color: #00ff41;
                    --border-color: #003b00;
                    font-family: 'JetBrains Mono', monospace;
                }
                body.sovereign-mode .header-btn,
                body.sovereign-mode .sidebar-item,
                body.sovereign-mode .stat-card {
                    border: 1px solid #003b00;
                    box-shadow: 0 0 5px rgba(0, 255, 65, 0.2);
                }
                body.sovereign-mode .log-entry {
                    color: #00ff41;
                    text-shadow: 0 0 2px #00ff41;
                }
                body.sovereign-mode .logo span {
                    color: #fff;
                }
            `;
            document.head.appendChild(style);
        }

        sovereignBtn.addEventListener("click", () => {
            document.body.classList.toggle("sovereign-mode");
            const isActive = document.body.classList.contains("sovereign-mode");
            if (isActive) {
                logToConsole("Sovereign Mode Engaged. Matrix protocols active.", "success");
                sovereignBtn.style.background = "rgba(0, 255, 65, 0.2)";
            } else {
                logToConsole("Sovereign Mode Disengaged.", "dim");
                sovereignBtn.style.background = "transparent";
            }
        });
    }

    // Font Size
    document.getElementById("font-inc").addEventListener("click", () => {
        currentFontSize += 2;
        document.documentElement.style.setProperty('--editor-font-size', currentFontSize + 'px');
    });

    document.getElementById("font-dec").addEventListener("click", () => {
        currentFontSize = Math.max(10, currentFontSize - 2);
        document.documentElement.style.setProperty('--editor-font-size', currentFontSize + 'px');
    });

    // Keyboard Shortcuts
    document.addEventListener("keydown", (e) => {
        if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
            e.preventDefault();
            runCode();
        }
    });
}

// --- System Monitor ---
function initSystemMonitor() {
    const pollInterval = 1000; // Faster polling for visualizer
    const canvas = document.getElementById('neural-canvas');
    const ctx = canvas ? canvas.getContext('2d') : null;

    // Waveform history
    const historySize = 30;
    let cpuHistory = new Array(historySize).fill(0);
    let neuralHistory = new Array(historySize).fill(0);

    function drawWaveform() {
        if (!ctx) return;
        const w = canvas.width;
        const h = canvas.height;
        ctx.clearRect(0, 0, w, h);

        // Draw Grid
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let i = 0; i < w; i += 40) { ctx.moveTo(i, 0); ctx.lineTo(i, h); }
        for (let j = 0; j < h; j += 40) { ctx.moveTo(0, j); ctx.lineTo(w, j); }
        ctx.stroke();

        // Draw CPU (Cyan)
        drawSmoothPath(cpuHistory, '#00d2ff', 2);

        // Draw Neural (Gold)
        drawSmoothPath(neuralHistory, '#ffd700', 3, true);
    }

    function drawSmoothPath(data, color, width, glow = false) {
        if (!ctx) return;
        ctx.beginPath();
        const w = canvas.width;
        const h = canvas.height;
        const step = w / (data.length - 1);

        // Spline interpolation for smoothness
        data.forEach((val, i) => {
            const x = i * step;
            const y = h - (val / 100) * h;
            if (i === 0) {
                ctx.moveTo(x, y);
            } else {
                // Simple linear for now, but robust
                ctx.lineTo(x, y);
            }
        });

        ctx.strokeStyle = color;
        ctx.lineWidth = width;

        if (glow) {
            ctx.shadowColor = color;
            ctx.shadowBlur = 15;
        } else {
            ctx.shadowBlur = 0;
        }

        ctx.stroke();
        ctx.shadowBlur = 0; // Reset

        // Fill area (optional, for neural)
        if (glow) {
            ctx.lineTo(w, h);
            ctx.lineTo(0, h);
            ctx.closePath();
            ctx.fillStyle = color + "11"; // Low opacity hex
            ctx.fill();
        }
    }

    async function updateStats() {
        try {
            const response = await fetch('/api/stats');
            if (!response.ok) return;
            const stats = await response.json();

            updateVal('stat-cpu', stats.cpu + '%', stats.cpu > 80);
            updateVal('stat-mem', stats.memory + '%', stats.memory > 80);
            updateVal('stat-disk', stats.disk + '%', stats.disk > 90);
            updateVal('stat-neural', stats.neural, false);

            // Update Histories
            cpuHistory.shift();
            cpuHistory.push(stats.cpu);
            neuralHistory.shift();
            neuralHistory.push(stats.neural);

            drawWaveform();

            // Adjust pulse speed based on neural activity
            const pulse = document.getElementById('monitor-pulse');
            if (pulse) {
                const duration = Math.max(0.2, 2 - (stats.neural / 50));
                pulse.style.animationDuration = `${duration}s`;
            }

            // Update System Health Indicator
            const statusEl = document.querySelector('.status-left span');
            if (statusEl) {
                let health = "OPTIMAL";
                let color = "#4caf50"; // Green

                if (stats.cpu > 90 || stats.memory > 90) {
                    health = "CRITICAL";
                    color = "#f44336"; // Red
                } else if (stats.cpu > 70 || stats.memory > 70) {
                    health = "ELEVATED";
                    color = "#ff9800"; // Orange
                } else if (stats.neural > 80) {
                    health = "HIGH RESONANCE";
                    color = "#ffd700"; // Gold
                }

                statusEl.textContent = `SYSTEM STATUS: ${health}`;
                statusEl.style.color = color;
                statusEl.style.fontWeight = "bold";
            }

            // Update Sys Info if available
            if (stats.sys_info) {
                 const versionEl = document.querySelector('.status-right span:first-child');
                 if (versionEl && !versionEl.dataset.updated) {
                     versionEl.textContent = `${stats.sys_info.platform} ${stats.sys_info.version}`;
                     versionEl.dataset.updated = "true";
                 }
            }

        } catch (e) {
            // Silent fail
        }
    }

    function updateVal(id, text, isHigh) {
        const el = document.getElementById(id);
        if (el) {
            el.textContent = text;
            if (isHigh) el.classList.add('high');
            else el.classList.remove('high');
        }
    }

    setInterval(updateStats, pollInterval);
    updateStats(); // Initial call
}

// Initialize
window.addEventListener("DOMContentLoaded", async () => {
    try {
        initEditor();
    } catch (e) {
        console.error("Editor initialization failed:", e);
        logToConsole("Editor failed to load. Please refresh.", "error");
    }

    try {
        setupUI();
    } catch (e) {
        console.error("UI setup failed:", e);
    }

    try {
        initSystemMonitor();
    } catch (e) {
        console.error("System Monitor initialization failed:", e);
    }

    logToConsole("Ark Environment Ready.", "success");

    // Load WASM module (non-blocking — UI is ready first)
    await initWasm();
});

