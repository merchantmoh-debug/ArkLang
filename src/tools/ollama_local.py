import json
from typing import Any, Dict, Optional
from urllib.parse import urlparse

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
    clean_host = host.rstrip('/')
    
    # SSRF Protection: Strict hostname validation
    try:
        parsed = urlparse(clean_host)
    except Exception:
        return f"[Security Block] Invalid host format: '{host}'"

    # Validate scheme
    if parsed.scheme not in ("http", "https"):
        return f"[Security Block] Scheme '{parsed.scheme}' not allowed. HTTP/HTTPS only."

    # Validate hostname (must be exactly localhost or 127.0.0.1)
    if parsed.hostname not in ("127.0.0.1", "localhost"):
        return f"[Security Block] Host '{parsed.hostname}' is not allowed. Localhost only."

    # Reconstruct URL safely
    # Note: We use the original clean_host structure but now we know the hostname is safe.
    # We could reconstruct from parsed, but clean_host is sufficient given the checks.
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
