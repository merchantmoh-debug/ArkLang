import { PROGRAMS } from '../programs.js';

export class TerminalElement extends HTMLElement {
    constructor() {
        super();
        this.history = [];
        this._queue = [];
        this.output = null;
    }

    connectedCallback() {
        const shadow = this.attachShadow({ mode: 'open' });
        shadow.innerHTML = `
            <style>
                :host {
                    display: block;
                    font-family: 'Courier New', monospace;
                    background: #0d1117;
                    border: 1px solid #30363d;
                    border-radius: 6px;
                    padding: 10px;
                    height: 400px;
                    overflow: hidden;
                    display: flex;
                    flex-direction: column;
                }
                .output {
                    flex-grow: 1;
                    overflow-y: auto;
                    margin-bottom: 10px;
                    color: #c9d1d9;
                }
                .input-line {
                    display: flex;
                    color: #58a6ff;
                }
                .prompt { margin-right: 8px; }
                input {
                    background: transparent;
                    border: none;
                    color: inherit;
                    width: 100%;
                    outline: none;
                    font-family: inherit;
                }
                .line { margin-bottom: 4px; }
                .system-msg { color: #8b949e; font-style: italic; }
                .signal-msg { color: #238636; font-weight: bold; }
                .error-msg { color: #f85149; }
            </style>
            <div class="output"></div>
            <div class="input-line">
                <span class="prompt">λ</span>
                <input type="text" autofocus spellcheck="false">
            </div>
        `;
        
        this.output = shadow.querySelector('.output');
        this.input = shadow.querySelector('input');
        
        this.input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                const cmd = this.input.value;
                this.addLine(`λ ${cmd}`);
                this.handleCommand(cmd);
                this.input.value = '';
            }
        });

        // Flush Queue
        if (this._queue.length > 0) {
            this._queue.forEach(q => this.addLine(q.text, q.className));
            this._queue = [];
        }
    }

    addLine(text, className = '') {
        if (!this.output) {
            this._queue.push({text, className});
            return;
        }
        const line = document.createElement('div');
        line.className = 'line ' + className;
        line.textContent = text;
        this.output.appendChild(line);
        this.scrollTop = this.scrollHeight;
    }

    log(text) {
        this.addLine(text, 'system-msg');
    }

    async handleCommand(cmd) {
        if (cmd === '(help)') {
            this.log('Available: (print "msg"), (run <prog>), (help)');
            this.log('Programs: factorial');
        } else if (cmd.startsWith('(print')) {
            const msg = cmd.match(/"(.*?)"/);
            if (msg) this.log(msg[1]);
            else this.log('Error: Invalid syntax');
        } else if (cmd.startsWith('(run')) {
            const progName = cmd.split(' ')[1].replace(')', '');
            const mast = PROGRAMS[progName];
            
            if (!mast) {
                this.log(`Error: Program '${progName}' not found.`);
                return;
            }

            this.log(`[Compiler] AST Loaded for '${progName}'.`);
            this.log(`[Runtime] Executing WASM Kernel...`);
            
            if (window.ark_eval) {
                // Signal Listener
                window.addEventListener('ark-signal', (e) => {
                    const msg = e.detail;
                    if (msg) {
                        this.addLine(`[Signal] ${msg.user}: ${msg.text}`, 'signal-msg');
                    }
                });

                window.addEventListener('ark-signal-ready', (e) => {
                   this.addLine(`[Signal] Uplink Established. PeerID: ${e.detail.peerId}`, 'system-msg');
                });

                try {
                    const json = JSON.stringify(mast);
                    const res = window.ark_eval(json);
                    this.log(res);
                } catch (e) {
                    this.log(`Runtime Error: ${e}`);
                }
            } else {
                this.log('Error: Runtime not loaded.');
            }

        } else {
            this.log("Error: Compiler offline (Use 'run' for pre-compiled binaries).");
        }
    }
}


customElements.define('ark-terminal', TerminalElement);
