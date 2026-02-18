# Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
#
# Swarm Bridge — Connects the Ark Compiler VM to the Agent Framework.
#
# This module serves as the bidirectional bridge between:
#   - The Rust VM (core/) which calls Python via Command::new("python3")
#   - The Agent Framework (src/) which orchestrates LLM-powered agents
#
# Usage:
#   python meta/swarm_bridge.py --task "Write a fibonacci function in Ark"
#   python meta/swarm_bridge.py --prompt "Explain linear types"

import os
import sys
import json
import argparse
import concurrent.futures
import time

# Resolve project root for imports
PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)


def _get_api_config():
    """Resolve API configuration from environment."""
    api_key = os.environ.get("GOOGLE_API_KEY") or os.environ.get("ARK_API_KEY")
    endpoint = os.environ.get("ARK_LLM_ENDPOINT", "")
    return api_key, endpoint


def ask_direct(prompt: str, model: str = "mistral") -> str:
    """Direct LLM call without agent framework. Used as fallback."""
    import urllib.request

    api_key, endpoint = _get_api_config()

    # Path 1: Local Ollama
    if endpoint:
        payload = json.dumps({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "stream": False
        }).encode("utf-8")
        req = urllib.request.Request(
            endpoint, data=payload,
            headers={"Content-Type": "application/json"}, method="POST"
        )
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            text = data.get("choices", [{}])[0].get("message", {}).get("content", "")
            if not text:
                text = data.get("message", {}).get("content", str(data))
            return text

    # Path 2: Gemini API
    if api_key:
        url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        payload = json.dumps({
            "contents": [{"parts": [{"text": prompt}]}]
        }).encode("utf-8")
        req = urllib.request.Request(
            f"{url}?key={api_key}", data=payload,
            headers={"Content-Type": "application/json"}, method="POST"
        )
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data["candidates"][0]["content"]["parts"][0]["text"]

    return "[Ark:Swarm] No API key configured. Set GOOGLE_API_KEY or ARK_LLM_ENDPOINT."


def ask_agent(prompt: str) -> str:
    """Route through the AgentOrchestrator if available."""
    try:
        from src.agent import AgentOrchestrator
        orchestrator = AgentOrchestrator()
        result = orchestrator.execute_task(prompt)
        if isinstance(result, dict):
            return result.get("response", str(result))
        return str(result)
    except ImportError:
        print("[Ark:Swarm] Agent framework unavailable, using direct call.", file=sys.stderr)
        return ask_direct(prompt)
    except Exception as e:
        print(f"[Ark:Swarm] Agent error ({e}), using direct call.", file=sys.stderr)
        return ask_direct(prompt)


def create_mission(task_name: str, instruction: str, persona: str = "") -> dict:
    """Create and execute a mission (task unit for swarm execution)."""
    full_prompt = f"{persona}\n\nTASK: {task_name}\nINSTRUCTION: {instruction}" if persona else instruction
    print(f"[Ark:Swarm] Executing: {task_name}")
    response = ask_agent(full_prompt)
    return {"task": task_name, "status": "complete", "response": response}


def run_swarm(tasks: list, max_workers: int = 4, persona: str = "") -> list:
    """Execute multiple tasks concurrently via the swarm."""
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = {}
        for task in tasks:
            name = task.get("name", "unnamed")
            instruction = task.get("instruction", "")
            future = executor.submit(create_mission, name, instruction, persona)
            futures[future] = name

        for future in concurrent.futures.as_completed(futures):
            task_name = futures[future]
            try:
                result = future.result()
                results.append(result)
                print(f"[Ark:Swarm] Completed: {task_name}")
            except Exception as e:
                results.append({"task": task_name, "status": "failed", "error": str(e)})
                print(f"[Ark:Swarm] Failed: {task_name} — {e}", file=sys.stderr)

    return results


def main():
    parser = argparse.ArgumentParser(description="Ark Swarm Bridge")
    parser.add_argument("--prompt", type=str, help="Single prompt to send to AI")
    parser.add_argument("--task", type=str, help="Task description for agent execution")
    parser.add_argument("--agent", type=str, default="coder", help="Agent type (coder, researcher, reviewer)")
    args = parser.parse_args()

    if args.prompt:
        print(ask_direct(args.prompt))
    elif args.task:
        print(ask_agent(args.task))
    else:
        # Default: read from stdin
        prompt = sys.stdin.read().strip()
        if prompt:
            print(ask_direct(prompt))
        else:
            print("[Ark:Swarm] No input provided. Use --prompt or --task.", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
