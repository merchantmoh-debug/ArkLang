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
                    font-family: 'JetBrains Mono', monospace;
                    background: rgba(13, 17, 23, 0.85);
                    backdrop-filter: blur(12px);
                    -webkit-backdrop-filter: blur(12px);
                    border: 1px solid rgba(0, 255, 65, 0.2);
                    border-radius: 8px;
                    padding: 0;
                    height: 100%;
                    min-height: 400px;
                    overflow: hidden;
                    display: flex;
                    flex-direction: column;
                    box-shadow: 0 0 20px rgba(0, 255, 65, 0.05);
                }
                .header {
                    background: rgba(0, 255, 65, 0.1);
                    border-bottom: 1px solid rgba(0, 255, 65, 0.2);
                    padding: 8px 12px;
                    font-size: 0.8em;
                    color: #00ff41;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    text-transform: uppercase;
                    letter-spacing: 1px;
                    user-select: none;
                }
                .status-dot {
                    display: inline-block;
                    width: 8px;
                    height: 8px;
                    background: #00ff41;
                    border-radius: 50%;
                    box-shadow: 0 0 5px #00ff41;
                    margin-right: 8px;
                }
                .content {
                    padding: 15px;
                    flex-grow: 1;
                    overflow-y: auto;
                    display: flex;
                    flex-direction: column;
                }
                .output {
                    flex-grow: 1;
                    margin-bottom: 10px;
                    color: #c9d1d9;
                }
                .input-line {
                    display: flex;
                    color: #00ff41;
                    align-items: center;
                }
                .prompt {
                    margin-right: 8px;
                    font-weight: bold;
                    text-shadow: 0 0 5px rgba(0, 255, 65, 0.5);
                }
                input {
                    background: transparent;
                    border: none;
                    color: inherit;
                    width: 100%;
                    outline: none;
                    font-family: inherit;
                    font-size: inherit;
                    text-shadow: 0 0 2px rgba(0, 255, 65, 0.3);
                }
                .line { margin-bottom: 4px; }
                .system-msg { color: #8b949e; font-style: italic; }
                .signal-msg { color: #238636; font-weight: bold; }
                .error-msg { color: #ff0055; }

                /* Scrollbar inside terminal */
                .content::-webkit-scrollbar { width: 6px; }
                .content::-webkit-scrollbar-track { background: transparent; }
                .content::-webkit-scrollbar-thumb { background: rgba(0, 255, 65, 0.2); border-radius: 3px; }
                .content::-webkit-scrollbar-thumb:hover { background: rgba(0, 255, 65, 0.5); }
            </style>
            <div class="header">
                <span><span class="status-dot"></span>Sovereign Terminal</span>
                <span>v112.0</span>
            </div>
            <div class="content">
                <div class="output"></div>
                <div class="input-line">
                    <span class="prompt">λ</span>
                    <input type="text" autofocus spellcheck="false">
                </div>
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

        // Scroll the container
        const container = this.shadowRoot.querySelector('.content');
        if (container) container.scrollTop = container.scrollHeight;
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
