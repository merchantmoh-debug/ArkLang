# SWARM MISSION: AGENT ALPHA (THE CORTEX)
**TARGET:** `meta/ark.py`
**OBJECTIVE:** Implement "Zero-Config" Ollama Detection.

## CONTEXT
Ark currently relies on `GOOGLE_API_KEY`. This violates sovereignty.
We must enable local inference via Ollama (`http://localhost:11434`) as the default.

## INSTRUCTIONS
1.  **Analyze `intrinsic_ask_ai`** in `meta/ark.py`.
2.  **Add Detection Logic:**
    - Perform a quick HTTP GET/POST to `http://localhost:11434/api/tags` or similar to check if Ollama is running.
    - If detected, set `ARK_AI_MODE = "OLLAMA"`.
3.  **Implement Fallback:**
    - IF Ollama is down -> Check `GOOGLE_API_KEY`.
    - IF No Key -> Use "Mock" Mode (so it doesn't crash).
4.  **Output Format:**
    - Ensure the function returns an `ArkValue` of type `String` containing the AI response.

## CONSTRAINTS
- **DO NOT** modify `compiler.ark` or `core/*`.
- **DO NOT** add external dependencies (use `urllib`, `json`).
