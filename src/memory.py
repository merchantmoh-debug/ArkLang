import json
import os
from typing import Any, Callable, Dict, List, Optional
from src.config import settings


class MemoryManager:
    """Simple JSON-file based memory manager for the agent."""

    def __init__(self, memory_file: str = settings.MEMORY_FILE):
        self.memory_file = memory_file
        self.summary: str = ""
        self._memory: List[Dict[str, Any]] = []
        self._load_memory()

    def _load_memory(self):
        """Loads memory from the JSON file if it exists."""
        self.summary = ""
        if os.path.exists(self.memory_file):
            try:
                with open(self.memory_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                if isinstance(data, dict):
                    self.summary = data.get("summary", "") or ""
                    history = data.get("history", [])
                    self._memory = history if isinstance(history, list) else []
                elif isinstance(data, list):
                    # Backward compatibility for legacy memory files
                    self._memory = data
                else:
                    print(f"Warning: Unexpected memory format in {self.memory_file}. Starting fresh.")
                    self._memory = []
            except json.JSONDecodeError:
                print(f"Warning: Could not decode memory file {self.memory_file}. Starting fresh.")
                self._memory = []
        else:
            self._memory = []

    def save_memory(self):
        """Saves the current memory state to the JSON file."""
        payload = {
            "summary": self.summary,
            "history": self._memory,
        }
        with open(self.memory_file, 'w', encoding='utf-8') as f:
            json.dump(payload, f, indent=2, ensure_ascii=False)

    def add_entry(self, role: str, content: str, metadata: Optional[Dict[str, Any]] = None):
        """Adds a new interaction to memory."""
        entry = {
            "role": role,
            "content": content,
            "metadata": metadata or {}
        }
        self._memory.append(entry)
        self.save_memory()

    def get_history(self) -> List[Dict[str, Any]]:
        """Returns the full conversation history."""
        return self._memory

    def _default_summarizer(self, old_messages: List[Dict[str, Any]], previous_summary: str) -> str:
        """
        Fallback summarization that compacts old messages.
        Concatenates previous summary (if any) with role-tagged message content.
        """
        lines: List[str] = []
        if previous_summary:
            lines.append(previous_summary.strip())
        for message in old_messages:
            role = message.get("role", "unknown")
            content = message.get("content", "")
            lines.append(f"{role}: {content}")
        return "\n".join(lines).strip()

    def get_context_window(
        self,
        system_prompt: str,
        max_messages: int,
        summarizer: Optional[Callable[[List[Dict[str, Any]], str], str]] = None
    ) -> List[Dict[str, str]]:
        """
        Returns the context window, applying a summary buffer when history exceeds max_messages.

        Args:
            system_prompt: The system prompt to prepend.
            max_messages: Maximum number of recent history messages to keep verbatim.
            summarizer: Callable that receives (old_messages, previous_summary) and returns a summary string.

        Raises:
            ValueError: If system_prompt is empty, max_messages is invalid, or summarizer returns non-string.
            TypeError: If summarizer does not accept the required arguments.
        """
        if not system_prompt:
            raise ValueError("system_prompt is required to build the context window.")
        if max_messages < 1:
            raise ValueError("max_messages must be at least 1.")

        history = self.get_history()
        system_message = {"role": "system", "content": system_prompt}

        if len(history) <= max_messages:
            return [system_message, *history]

        summarizer_fn = summarizer or self._default_summarizer
        messages_to_summarize = [dict(msg) for msg in history[:-max_messages]]
        recent_history = [dict(msg) for msg in history[-max_messages:]]

        try:
            new_summary = summarizer_fn(messages_to_summarize, self.summary)
        except TypeError as exc:
            raise TypeError("Summarizer must accept two arguments: (old_messages, previous_summary).") from exc

        if not isinstance(new_summary, str):
            raise ValueError("Summarizer must return a string.")

        previous_summary = self.summary
        self.summary = new_summary.strip()
        if self.summary != previous_summary:
            self.save_memory()

        summary_message = {
            "role": "system",
            "content": f"Previous Summary: {self.summary}"
        }

        return [system_message, summary_message, *recent_history]

    def clear_memory(self):
        """Clears the agent's memory."""
        self._memory = []
        self.summary = ""
        self.save_memory()
