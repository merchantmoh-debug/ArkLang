import { PROGRAMS } from '../programs.js';

export class TerminalElement extends HTMLElement {
    constructor() {
        super();
        this.history = [];
    }
    // ... connectedCallback stays same ...

    // ... log stays same ...

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
