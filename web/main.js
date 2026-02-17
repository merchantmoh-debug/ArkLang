import {EditorView, basicSetup} from "codemirror"
import {keymap} from "@codemirror/view"
import {Compartment} from "@codemirror/state"
import {defaultKeymap, indentWithTab} from "@codemirror/commands"
import {StreamLanguage} from "@codemirror/language"
import {oneDark} from "@codemirror/theme-one-dark"

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
    "Hello World": `func main() {
    print("Hello, Ark Sovereign World!");
}`,
    "Fibonacci": `func fib(n) {
    if n <= 1 { return n; }
    return fib(n - 1) + fib(n - 2);
}

func main() {
    let n = 10;
    print("Fibonacci of " + str(n) + " is " + str(fib(n)));
}`,
    "Linked List": `struct Node {
    value
    next
}

func main() {
    let head = Node(1, Node(2, Node(3, nil)));

    let current = head;
    while current != nil {
        print(current.value);
        current = current.next;
    }
}`,
    "HTTP Request": `func main() {
    let response = http.get("https://api.ark.io/status");
    print("Status: " + str(response.status));
    print("Body: " + response.body);
}`,
    "Crypto": `func main() {
    let data = "Sovereign Data";
    let hash = crypto.sha256(data);
    print("SHA-256: " + hash);
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
    const outputDiv = document.getElementById("console-output");

    // Add "Running..." marker
    logToConsole("Running on Sovereign Runtime...", "info", true);

    try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 10000); // 10s timeout

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

        if (result.stdout) logToConsole(result.stdout);
        if (result.stderr) logToConsole(result.stderr, "error");
        if (result.error) logToConsole(result.error, "error");

    } catch (e) {
        if (e.name === 'AbortError') {
            logToConsole("Execution timed out (10s limit).", "error");
        } else {
            logToConsole(`Failed to execute: ${e.message}`, "error");
            logToConsole("Make sure the Ark server is running: `python meta/ark.py serve`", "dim");
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
    const pollInterval = 2000;

    async function updateStats() {
        try {
            const response = await fetch('/api/stats');
            if (!response.ok) return;
            const stats = await response.json();

            updateVal('stat-cpu', stats.cpu + '%', stats.cpu > 80);
            updateVal('stat-mem', stats.memory + '%', stats.memory > 80);
            updateVal('stat-disk', stats.disk + '%', stats.disk > 90);
            updateVal('stat-neural', stats.neural, false);

            // Adjust pulse speed based on neural activity
            const pulse = document.getElementById('monitor-pulse');
            if (pulse) {
                const duration = Math.max(0.5, 2 - (stats.neural / 100));
                pulse.style.animationDuration = `${duration}s`;
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
window.addEventListener("DOMContentLoaded", () => {
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
});
