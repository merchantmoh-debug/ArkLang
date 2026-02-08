import { injectP2P } from 'https://esm.sh/konomi-p2p@1.0.1';

export class Network {
    constructor() {
        this.p2p = null;
        this.messages = null;
        this.username = "Sovereign-" + Math.floor(Math.random() * 1000);
        this.init();
    }

    async init() {
        try {
            console.log("[Signal] Initializing P2P Swarm...");
            this.p2p = await injectP2P({
                roomId: 'ark-global-v1',
                user: { name: this.username }
            });

            this.messages = this.p2p.getSharedArray('ark-signal-log');
            
            // Listen for new messages
            this.messages.observe(() => {
                // This will trigger a UI update if we hook into it
                // For now, we rely on the terminal polling or event dispatch
                const latest = this.messages.get(this.messages.length - 1);
                if (latest) {
                    window.dispatchEvent(new CustomEvent('ark-signal', { detail: latest }));
                }
            });

            console.log("[Signal] Connected to Swarm.");
            window.dispatchEvent(new CustomEvent('ark-signal-ready', { detail: { peerId: this.p2p.peerId } }));
            
        } catch (e) {
            console.error("[Signal] Connection Failed:", e);
        }
    }

    broadcast(text) {
        if (!this.messages) return;
        
        this.messages.push([{
            user: this.username,
            text: text,
            time: Date.now()
        }]);
    }
}

// Global Singleton
window.ArkNetwork = new Network();
