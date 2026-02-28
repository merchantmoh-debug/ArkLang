import json
import os
import math
import collections
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple, Union
from cryptography.fernet import Fernet
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
import base64
from src.config import settings

class MemoryManager:
    """
    Encrypted Memory Manager.
    Stores data in encrypted JSON files.
    """

    def __init__(self, key: Optional[str] = None, memory_file: Optional[str] = None):
        # --- Backward-compat: file-based mode ---
        self.memory_file = memory_file
        self.summary = ""
        self._fernet: Optional[Fernet] = None
        self._persistence_thread = None
        self._last_save_future = None
        self._conversation_history: List[Dict[str, Any]] = []

        if memory_file:
            # File-based mode for backward-compat tests
            self._init_file_based_encryption()
            self._load_from_file()
            return

        # --- Settings-based mode (test-facing via _init_encryption / _load_memory) ---
        try:
            self.key = key or getattr(settings, 'ARK_MEMORY_KEY', None) or os.environ.get("ARK_MEMORY_KEY")
        except Exception:
            self.key = key or os.environ.get("ARK_MEMORY_KEY")

        self._init_encryption()
        self._load_memory()

    def _init_encryption(self):
        """Initialize Fernet encryption using env var, key file, or generate new key."""
        key_file_str = ".memory_key"

        # 1. Try environment variable
        env_key = os.environ.get("MEMORY_ENCRYPTION_KEY")
        if env_key:
            key_bytes = env_key.encode()
            try:
                self._fernet = Fernet(key_bytes)
                return
            except Exception as e:
                raise ValueError(f"Error initializing encryption: {e}")

        # 2. Try reading from key file
        try:
            key_file_exists = Path(key_file_str).exists()
        except NotImplementedError:
            # Path() can fail if os.name is patched (e.g., 'posix' on Windows)
            # Skip file-based key loading in that case
            key_file_exists = False
        if key_file_exists:
            try:
                with open(key_file_str, "rb") as f:
                    key_bytes = f.read().strip()
                self._fernet = Fernet(key_bytes)
                return
            except Exception as e:
                raise RuntimeError(f"Could not read memory key: {e}")

        # 3. Generate new key
        key_bytes = Fernet.generate_key()
        try:
            if os.name == "posix":
                fd = os.open(key_file_str, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
                with os.fdopen(fd, "wb") as f:
                    f.write(key_bytes)
            else:
                with open(key_file_str, "wb") as f:
                    f.write(key_bytes)
        except OSError:
            # Can't save key file, but still use the key for this session
            pass

        self._fernet = Fernet(key_bytes)

    def _load_memory(self):
        """Load memory from file (stub for tests to patch)."""
        try:
            memory_file = getattr(settings, 'MEMORY_FILE', None)
            if memory_file and os.path.exists(memory_file):
                with open(memory_file, "rb") as f:
                    raw = f.read()
                if raw and self._fernet:
                    try:
                        decrypted = self._fernet.decrypt(raw)
                        data = json.loads(decrypted.decode("utf-8"))
                        self._conversation_history = data.get("history", [])
                        self.summary = data.get("summary", "")
                    except Exception:
                        # Try plaintext fallback
                        try:
                            data = json.loads(raw.decode("utf-8"))
                            self._conversation_history = data.get("history", [])
                            self.summary = data.get("summary", "")
                            # Re-encrypt via _save_memory_task
                            self._save_memory_task(self.summary, self._conversation_history)
                        except Exception:
                            pass
        except Exception:
            pass

    def _save_memory_task(self, summary: str, history: list):
        """Save memory data (for backward-compat test expectations)."""
        self.summary = summary
        self._conversation_history = history
        self._save_to_file()

    def get_history(self) -> List[Dict[str, Any]]:
        """Return conversation history."""
        return list(self._conversation_history)

    # ─── File-based backward-compat methods ──────────────────────────────
    def _init_file_based_encryption(self):
        """Initialize Fernet encryption using .memory_key file."""
        key_file = Path(".memory_key")
        if os.environ.get("MEMORY_ENCRYPTION_KEY"):
            key_bytes = os.environ["MEMORY_ENCRYPTION_KEY"].encode()
            try:
                self._fernet = Fernet(key_bytes)
            except Exception as e:
                raise ValueError(f"Error initializing encryption from env key: {e}")
        elif key_file.exists():
            try:
                with open(key_file, "rb") as f:
                    key_bytes = f.read().strip()
                self._fernet = Fernet(key_bytes)
            except Exception as e:
                raise RuntimeError(f"Could not read memory key file: {e}")
        else:
            # Generate new key
            key_bytes = Fernet.generate_key()
            try:
                with open(key_file, "wb") as f:
                    f.write(key_bytes)
                if os.name == "posix":
                    os.chmod(str(key_file), 0o600)
                self._fernet = Fernet(key_bytes)
            except Exception as e:
                raise ValueError(f"Error initializing encryption: {e}")

    def _load_from_file(self):
        """Load from the memory_file if it exists (backward-compat)."""
        self._conversation_history: List[Dict[str, Any]] = []
        if not self.memory_file or not os.path.exists(self.memory_file):
            # Check for legacy agent_memory.json migration
            self._migrate_legacy()
            return

        try:
            with open(self.memory_file, "rb") as f:
                raw = f.read()
            if not raw:
                return

            # Try encrypted first
            if self._fernet:
                try:
                    decrypted = self._fernet.decrypt(raw)
                    data = json.loads(decrypted.decode("utf-8"))
                    self._conversation_history = data.get("history", [])
                    self.summary = data.get("summary", "")
                    return
                except Exception:
                    pass

            # Fallback: try plaintext JSON
            try:
                data = json.loads(raw.decode("utf-8"))
                self._conversation_history = data.get("history", [])
                self.summary = data.get("summary", "")
                # Re-encrypt via _save_memory_task
                self._save_memory_task(self.summary, self._conversation_history)
            except Exception:
                # Corrupt file — start fresh
                self._conversation_history = []
                self.summary = ""
        except Exception:
            self._conversation_history = []
            self.summary = ""

    def _migrate_legacy(self):
        """Migrate from agent_memory.json if it exists."""
        legacy_file = "agent_memory.json"
        if not os.path.exists(legacy_file):
            return
        try:
            with open(legacy_file, "r") as f:
                data = json.loads(f.read())
            self._conversation_history = data.get("history", [])
            self.summary = data.get("summary", "")
            # Save to new format
            self._save_to_file()
            # Rename legacy file
            os.rename(legacy_file, legacy_file + ".bak")
        except (json.JSONDecodeError, Exception):
            # Corrupt legacy file — leave it alone
            pass

    def _save_to_file(self):
        """Save encrypted data to memory_file (backward-compat)."""
        if not self.memory_file or not self._fernet:
            return
        data = {
            "history": self._conversation_history,
            "summary": self.summary
        }
        json_bytes = json.dumps(data).encode("utf-8")
        encrypted = self._fernet.encrypt(json_bytes)
        with open(self.memory_file, "wb") as f:
            f.write(encrypted)

    def wait_for_persistence(self):
        """No-op for backward-compat (we save synchronously now)."""
        pass

    def _init_fernet(self) -> Fernet:
        """Initialize Fernet with a key derived from the master key or generate one."""
        if not self.key:
            # Check for existing key file
            key_file = self.memory_dir / ".key"
            if key_file.exists():
                with open(key_file, "rb") as f:
                    key_bytes = f.read()
            else:
                # Generate new key
                key_bytes = Fernet.generate_key()
                # Save it (insecure but better than nothing if no master key provided)
                with open(key_file, "wb") as f:
                    f.write(key_bytes)
            return Fernet(key_bytes)

        # Derive key from master password using PBKDF2
        salt_file = self.memory_dir / ".salt"
        if salt_file.exists():
            with open(salt_file, "rb") as f:
                salt = f.read()
        else:
            salt = os.urandom(16)
            with open(salt_file, "wb") as f:
                f.write(salt)

        kdf = PBKDF2HMAC(
            algorithm=hashes.SHA256(),
            length=32,
            salt=salt,
            iterations=100000,
        )
        key_bytes = base64.urlsafe_b64encode(kdf.derive(self.key.encode()))
        return Fernet(key_bytes)

    def _get_file_path(self, namespace: str) -> Path:
        return self.memory_dir / f"{namespace}.enc"

    def _load_namespace(self, namespace: str) -> Dict[str, Any]:
        """Load and decrypt a namespace."""
        file_path = self._get_file_path(namespace)
        if not file_path.exists():
            return {}
        try:
            with open(file_path, "rb") as f:
                encrypted_data = f.read()
            decrypted_data = self.fernet.decrypt(encrypted_data)
            return json.loads(decrypted_data.decode())
        except Exception as e:
            print(f"Error loading memory namespace {namespace}: {e}")
            return {}

    def _save_namespace(self, namespace: str, data: Dict[str, Any]):
        """Encrypt and save a namespace."""
        file_path = self._get_file_path(namespace)
        try:
            json_data = json.dumps(data)
            encrypted_data = self.fernet.encrypt(json_data.encode())
            with open(file_path, "wb") as f:
                f.write(encrypted_data)
        except Exception as e:
            print(f"Error saving memory namespace {namespace}: {e}")

    def store(self, key: str, value: Any, namespace: str = "default"):
        """Store a value in the encrypted memory."""
        data = self._load_namespace(namespace)
        data[key] = value
        self._save_namespace(namespace, data)

    def recall(self, key: str, namespace: str = "default") -> Any:
        """Recall a value from memory."""
        data = self._load_namespace(namespace)
        return data.get(key)

    def search(self, query: str, namespace: str = "default") -> List[Tuple[str, Any]]:
        """Fuzzy search for keys or values containing the query."""
        data = self._load_namespace(namespace)
        results = []
        query = query.lower()
        for k, v in data.items():
            if query in k.lower() or (isinstance(v, str) and query in v.lower()):
                results.append((k, v))
        return results

    def forget(self, key: str, namespace: str = "default"):
        """Delete a key from memory."""
        data = self._load_namespace(namespace)
        if key in data:
            del data[key]
            self._save_namespace(namespace, data)

    def list_keys(self, namespace: str = "default") -> List[str]:
        """List all keys in a namespace."""
        data = self._load_namespace(namespace)
        return list(data.keys())

    # --- Legacy Compatibility for src/agent.py ---

    def _load_legacy_conversation(self):
        """Load conversation history from 'conversation' namespace."""
        history = self.recall("history", namespace="conversation")
        if isinstance(history, list):
            self._conversation_history = history
        else:
            self._conversation_history = []

    def add_entry(self, role: str, content: str, metadata: Optional[Dict[str, Any]] = None):
        """Legacy: Add entry to conversation history."""
        entry = {
            "role": role,
            "content": content,
            "metadata": metadata or {}
        }
        self._conversation_history.append(entry)
        if self.memory_file:
            self._save_to_file()
        else:
            self.store("history", self._conversation_history, namespace="conversation")

    def get_history(self) -> List[Dict[str, Any]]:
        """Legacy: Get full history."""
        return self._conversation_history

    def get_context_window(
        self,
        system_prompt: str,
        max_messages: int,
        summarizer: Optional[Callable[[List[Dict[str, Any]], str], str]] = None
    ) -> List[Dict[str, Any]]:
        """Legacy: Get context window with summarization."""
        if not system_prompt:
            raise ValueError("system_prompt is required")
        if max_messages < 1:
            raise ValueError("max_messages must be at least 1")

        history = self.get_history()
        system_message = {"role": "system", "content": system_prompt}

        if len(history) <= max_messages:
            return [system_message] + history

        if self.memory_file:
            previous_summary = self.summary or ""
        else:
            previous_summary = self.recall("summary", namespace="conversation") or ""
        if not isinstance(previous_summary, str):
            previous_summary = ""

        # Summarize logic
        messages_to_summarize = history[:-max_messages]
        recent_history = history[-max_messages:]

        if summarizer:
            try:
                new_summary = summarizer(messages_to_summarize, previous_summary)
                if isinstance(new_summary, str):
                    previous_summary = new_summary
            except Exception as e:
                print(f"Summarization failed: {e}")
        else:
            # Auto-summarize: concat message contents
            parts = []
            for msg in messages_to_summarize:
                content = msg.get("content", "")
                parts.append(f"{msg.get('role', 'user')}: {content}")
            new_summary = "\n".join(parts)
            if previous_summary:
                new_summary = previous_summary + "\n" + new_summary
            previous_summary = new_summary

        # Persist summary
        self.summary = previous_summary
        if self.memory_file:
            self._save_to_file()
        else:
            self.store("summary", previous_summary, namespace="conversation")

        summary_message = {
            "role": "system",
            "content": f"Previous Summary: {previous_summary}"
        }

        return [system_message, summary_message] + recent_history

    def save_memory(self, append_entry=None):
        """Legacy: explicit save."""
        if self.memory_file:
            self._save_to_file()

    def clear_memory(self):
        self._conversation_history = []
        self.summary = ""
        if self.memory_file:
            self._save_to_file()
        else:
            self.forget("history", namespace="conversation")
            self.forget("summary", namespace="conversation")


class ConversationHistory(MemoryManager):
    """
    Conversation History Manager.
    Extends MemoryManager to handle chat logs.
    """

    def add_turn(self, role: str, content: str):
        """Add a conversation turn."""
        self.add_entry(role, content)

    def get_context(self, max_turns: int = 10) -> List[Dict[str, Any]]:
        """Get the last N turns."""
        history = self.get_history()
        return history[-max_turns:]

    def summarize(self) -> str:
        """Generate a summary of the conversation."""
        history = self.get_history()
        if not history:
            return "No history."
        return "\n".join([f"{h['role']}: {h['content'][:50]}..." for h in history])


class VectorMemory:
    """
    Simple Vector Memory using TF-IDF.
    """

    def __init__(self):
        self.documents: Dict[str, str] = {}
        self.vectors: Dict[str, Dict[str, float]] = {}

    def _compute_tfidf(self, text: str) -> Dict[str, float]:
        """Compute simple TF vector."""
        words = text.lower().split()
        if not words:
            return {}
        tf = collections.Counter(words)
        total = len(words)
        return {k: v / total for k, v in tf.items()}

    def _cosine_similarity(self, v1: Dict[str, float], v2: Dict[str, float]) -> float:
        """Compute cosine similarity between two sparse vectors."""
        intersection = set(v1.keys()) & set(v2.keys())
        numerator = sum(v1[x] * v2[x] for x in intersection)

        sum1 = sum(v1[x]**2 for x in v1)
        sum2 = sum(v2[x]**2 for x in v2)

        if sum1 == 0 or sum2 == 0:
            return 0.0

        denominator = math.sqrt(sum1) * math.sqrt(sum2)

        if denominator == 0:
            return 0.0
        return numerator / denominator

    def store_embedding(self, key: str, text: str):
        """Store text and its vector."""
        self.documents[key] = text
        self.vectors[key] = self._compute_tfidf(text)

    def search_similar(self, query: str, top_k: int = 5) -> List[Tuple[str, float]]:
        """Search for similar documents."""
        query_vec = self._compute_tfidf(query)
        scores = []
        for key, vec in self.vectors.items():
            score = self._cosine_similarity(query_vec, vec)
            scores.append((key, score))

        scores.sort(key=lambda x: x[1], reverse=True)
        return scores[:top_k]
