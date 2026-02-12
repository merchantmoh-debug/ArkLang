# SWARM MISSION: AGENT BETA (THE ARCHITECT)
**TARGET:** `apps/sovereign_shell.ark`
**OBJECTIVE:** Create a Voice-Activated Terminal Assistant.

## CONTEXT
We need a "Killer App" to demonstrate Ark's capabilities.
The "Sovereign Shell" will allow users to control their machine via natural language.

## INSTRUCTIONS
1.  **Create `apps/sovereign_shell.ark`**.
2.  **Implement Main Loop:**
    - Print a welcome message ("🔮 SOVEREIGN SHELL ONLINE").
    - Loop forever:
        - Read input (Text for now, `sys.io.read_line()`).
        - Send input to `intrinsic_ask_ai()` with a system prompt: "You are a shell assistant. Translate this user request into a system command or an Ark script."
        - Print the AI's response.
        - (Optional) execute the response if safe.
3.  **Style:**
    - Use Emoji and "Cyberpunk" aesthetics in the output.

## CONSTRAINTS
- **DO NOT** modify `meta/ark.py`.
- Rely on `intrinsic_ask_ai` key/logic being handled by Agent Alpha.
