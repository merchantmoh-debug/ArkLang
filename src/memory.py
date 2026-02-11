import json
import os
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional
from cryptography.fernet import Fernet
from src.config import settings


class MemoryManager:
    """Simple JSON-file based memory manager for the agent."""

    def __init__(self, memory_file: str = settings.MEMORY_FILE):
        self.memory_file = memory_file
        self.summary: str = ""
        self._memory: List[Dict[str, Any]] = []
        self._fernet: Optional[Fernet] = None
        self._init_encryption()
        self._load_memory()

    def _init_encryption(self):
        """Initializes the encryption key and Fernet instance."""
        key = os.environ.get("MEMORY_ENCRYPTION_KEY")
        key_file = Path(".memory_key")

        if not key:
            if key_file.exists():
                try:
                    with open(key_file, "rb") as f:
                        key = f.read().strip()
                except Exception as e:
                    raise RuntimeError(f"Could not read memory key from {key_file}: {e}")

            if not key:
                print("Generating new encryption key for memory...")
                key = Fernet.generate_key()
                try:
                    # restrictive permissions (0o600) only work on POSIX, but harmless on Windows
                    if os.name == 'posix':
                        # Atomic creation with restricted permissions
                        fd = os.open(key_file, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
                        with os.fdopen(fd, "wb") as f:
                            f.write(key)
                    else:
                        with open(key_file, "wb") as f:
                            f.write(key)
                except Exception as e:
                    print(f"Warning: Could not save memory key to {key_file}: {e}")
                    # We continue with in-memory key, which is secure for this session but not persistent.
                    # This is acceptable (better than crashing if FS is read-only).

        if isinstance(key, str):
            key = key.encode()

        try:
            self._fernet = Fernet(key)
        except Exception as e:
            # FAIL CLOSED: Do not continue without valid encryption
            raise ValueError(f"Error initializing encryption: {e}")

    def _load_memory(self):
        """Loads memory from the encrypted file (or legacy JSON if present)."""
        self.summary = ""

        # Check for legacy plaintext file first if we are using the new default extension
        legacy_file = "agent_memory.json"
        if not os.path.exists(self.memory_file) and os.path.exists(legacy_file):
            print(f"Found legacy memory file: {legacy_file}. Migrating to encrypted storage...")
            try:
                with open(legacy_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                self._process_loaded_data(data)
                # If successful, save immediately to encrypted file
                self.save_memory()
                # Rename legacy file to .bak
                os.rename(legacy_file, legacy_file + ".bak")
                print(f"Migration complete. Legacy file moved to {legacy_file}.bak")
                return
            except Exception as e:
                print(f"Error migrating legacy memory: {e}")
                # Fall through to normal load attempt

        if os.path.exists(self.memory_file):
            try:
                with open(self.memory_file, 'rb') as f:
                    file_content = f.read()

                # Attempt to decrypt
                if self._fernet:
                    try:
                        decrypted_content = self._fernet.decrypt(file_content)
                        data = json.loads(decrypted_content.decode('utf-8'))
                    except Exception:
                        # Fallback: maybe it's a plaintext file with the new name?
                        # Or the key changed?
                        try:
                            data = json.loads(file_content.decode('utf-8'))
                            print("Warning: Memory file was plaintext. Saving as encrypted now.")
                            self._process_loaded_data(data)
                            self.save_memory()
                            return
                        except json.JSONDecodeError:
                            print(f"Error: Could not decrypt or decode memory file {self.memory_file}.")
                            data = None
                else:
                    # Should be unreachable if _init_encryption works correctly,
                    # but defensively handle it.
                    raise RuntimeError("Encryption not initialized.")

                if data is not None:
                    self._process_loaded_data(data)
                else:
                     self._memory = []

            except Exception as e:
                print(f"Warning: Failed to load memory from {self.memory_file}: {e}")
                self._memory = []
        else:
            self._memory = []

    def _process_loaded_data(self, data):
        """Helper to process the raw loaded data structure."""
        if isinstance(data, dict):
            self.summary = data.get("summary", "") or ""
            history = data.get("history", [])
            self._memory = history if isinstance(history, list) else []
        elif isinstance(data, list):
            # Backward compatibility for legacy memory files
            self._memory = data
        else:
            print(f"Warning: Unexpected memory format. Starting fresh.")
            self._memory = []

    def save_memory(self):
        """Saves the current memory state to the encrypted file."""
        payload = {
            "summary": self.summary,
            "history": self._memory,
        }

        # FAIL CLOSED: Ensure encryption is available
        if not self._fernet:
            raise RuntimeError("Encryption not initialized. Cannot save memory securely.")

        try:
            json_str = json.dumps(payload, indent=2, ensure_ascii=False)
            data_bytes = json_str.encode('utf-8')

            # Strictly encrypt
            encrypted_data = self._fernet.encrypt(data_bytes)
            with open(self.memory_file, 'wb') as f:
                f.write(encrypted_data)

        except Exception as e:
            print(f"Error saving memory: {e}")
            # For save operations, logging is preferred to crashing the agent loop,
            # but we ensure no plaintext is written by failing closed above.
            pass

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
