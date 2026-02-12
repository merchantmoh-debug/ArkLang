import json
from typing import Any, Dict, Optional

import requests


def call_local_ollama(
    prompt: str,
    model: str = "qwen3:0.6b",
    host: str = "http://127.0.0.1:11434",
    stream: bool = False,
    options: Optional[Dict[str, Any]] = None,
) -> str:
    """
    Call a local Ollama-style endpoint at /api/generate.
    
    Security: Restricts 'host' to generic local loopback addresses to prevent SSRF.
    """
    # SSRF Protection: Only allow local loopback
    allowed_hosts = ["http://127.0.0.1", "http://localhost", "https://127.0.0.1", "https://localhost"]
    clean_host = host.rstrip('/')
    
    # Check if host starts with any allowed prefix
    if not any(clean_host.startswith(allowed) for allowed in allowed_hosts):
         # Also allow specific port 11434 checks if needed, but simplest is prefix
         # Actually, let's be strict.
         if not (clean_host == "http://127.0.0.1:11434" or clean_host == "http://localhost:11434"):
             # Fallback for custom ports on localhost: check hostname parsing
             from urllib.parse import urlparse
             parsed = urlparse(clean_host)
             if parsed.hostname not in ("127.0.0.1", "localhost"):
                  return f"[Security Block] Host '{host}' is not allowed. Localhost only."

    url = f"{clean_host}/api/generate"
    payload = {
        "model": model,
        "prompt": prompt,
        "stream": stream,
    }
    if options:
        payload["options"] = options

    try:
        resp = requests.post(url, json=payload, timeout=60)
        resp.raise_for_status()
        data = resp.json()
    except Exception as exc:
        return f"[call_local_ollama] request failed: {exc}"

    # Ollama /api/generate responses may contain 'response' or 'output' fields
    text = data.get("response") or data.get("output") or data
    if not isinstance(text, str):
        try:
            text = json.dumps(text, ensure_ascii=False)
        except Exception:
            text = str(text)
    return text.strip()
