export class HeroElement extends HTMLElement {
    connectedCallback() {
        this.innerHTML = `
            <div class="glass-panel">
                <h1>Ark::Sovereign_Runtime</h1>
                <p>
                    > version: v1.0.0<br>
                    > status: <span style="color:cyan">ONLINE</span><br>
                    > architecture: NEURO-SYMBOLIC
                </p>
                <p>
                    <i>"We do not ask for freedom. We compile it."</i>
                </p>
                <div style="margin-top: 10px;">
                    <a href="https://github.com/merchantmoh-debug/ark-compiler">Source Code</a> | 
                    <a href="LICENSE">AGPLv3</a>
                </div>
            </div>
        `;
    }
}

customElements.define('ark-hero', HeroElement);
