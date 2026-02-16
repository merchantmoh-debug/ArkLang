import asyncio
import inspect
import json
import logging
import os
import time
import traceback
from datetime import datetime
from typing import Any, Callable, Dict, List, Optional, Union

# Try importing dependencies, handle gracefully if missing
try:
    from google import genai
    GOOGLE_GENAI_AVAILABLE = True
except ImportError:
    GOOGLE_GENAI_AVAILABLE = False

try:
    from src.config import settings
    from src.tools.openai_proxy import call_openai_chat
    from src.utils.dummy_client import DummyClient
except ImportError:
    # Fallback for direct execution/testing without full project structure
    class Settings:
        GOOGLE_API_KEY = os.getenv("GOOGLE_API_KEY", "")
        OPENAI_BASE_URL = os.getenv("OPENAI_BASE_URL", "")
        OPENAI_API_KEY = os.getenv("OPENAI_API_KEY", "")
        GEMINI_MODEL_NAME = "gemini-2.0-flash-exp"
        OPENAI_MODEL = "gpt-4o"
        DEBUG_MODE = True
        AGENT_NAME = "ArkAgent"

    settings = Settings()

    def call_openai_chat(*args, **kwargs):
        return "OpenAI Proxy Not Available"

    class DummyClient:
        def __init__(self, response_text="Task completed"):
            self.response_text = response_text
        class Models:
            def __init__(self, text):
                self.text = text
            def generate_content(self, *args, **kwargs):
                return type('obj', (object,), {'text': self.text})
        def __init__(self, response_text="Task completed"):
            self.models = self.Models(response_text)

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S"
)

class BaseAgent:
    """
    Base Agent class implementing the core ReAct loop, tool management,
    and LLM interaction logic with retry mechanisms.
    """
    
    def __init__(
        self,
        name: str,
        model: str = None,
        system_prompt: str = "You are a helpful AI assistant.",
        tools: List[Callable] = None,
        memory: Any = None
    ):
        self.name = name
        self.model = model or settings.GEMINI_MODEL_NAME
        self.system_prompt = system_prompt
        self.memory = memory or []  # Simple list for memory if not provided
        self.tools: Dict[str, Callable] = {}
        self.logger = logging.getLogger(self.name)

        # Token usage tracking
        self.token_usage = {"input": 0, "output": 0, "total": 0}
        
        # Initialize Client
        self._init_client()

        # Initialize tools
        if tools:
            for tool in tools:
                self.add_tool(tool)

    def _init_client(self):
        """Initialize the appropriate LLM client based on configuration."""
        self.client = None
        self.client_type = "dummy"

        if settings.GOOGLE_API_KEY and GOOGLE_GENAI_AVAILABLE:
            try:
                self.client = genai.Client(api_key=settings.GOOGLE_API_KEY)
                self.client_type = "google"
                self.logger.info(f"Initialized Google GenAI client with model {self.model}")
            except Exception as e:
                self.logger.error(f"Failed to initialize Google GenAI: {e}")
        
        if not self.client and settings.OPENAI_BASE_URL:
            self.client_type = "openai"
            # If using OpenAI proxy, we might want to default to the configured OpenAI model
            if self.model == settings.GEMINI_MODEL_NAME:
                self.model = settings.OPENAI_MODEL
            self.logger.info(f"Initialized OpenAI compatible client with model {self.model}")
            
        if not self.client and self.client_type == "dummy":
            self.client = DummyClient()
            self.logger.warning("Using DummyClient (No API keys found)")

    def add_tool(self, tool: Callable):
        """Register a tool for the agent to use."""
        if not callable(tool):
            raise ValueError(f"Tool {tool} must be callable")

        # Get name from function name or __name__
        if hasattr(tool, "__name__"):
            name = tool.__name__
        else:
            name = str(tool)

        self.tools[name] = tool
        self.logger.debug(f"Registered tool: {name}")

    def log(self, level: str, message: str):
        """Structured logging with timestamp."""
        timestamp = datetime.now().isoformat()
        log_msg = f"[{timestamp}] [{self.name}] {message}"
        
        lvl = level.upper()
        if lvl == "DEBUG":
            self.logger.debug(message)
        elif lvl == "INFO":
            self.logger.info(message)
        elif lvl == "WARNING":
            self.logger.warning(message)
        elif lvl == "ERROR":
            self.logger.error(message)
        elif lvl == "CRITICAL":
            self.logger.critical(message)
        else:
            self.logger.info(f"[{level}] {message}")

    def _estimate_tokens(self, text: str) -> int:
        """Simple estimation of tokens (char count / 4)."""
        if not text:
            return 0
        return len(str(text)) // 4

    async def _call_llm(self, messages: List[Dict[str, str]], tools_schema: Optional[str] = None) -> str:
        """
        Internal method to call the LLM with retries and exponential backoff.
        """
        max_retries = 3
        backoff_factor = 2
        
        prompt_text = ""
        # Convert messages to prompt string for simple APIs
        for msg in messages:
            role = msg.get("role", "user").upper()
            content = msg.get("content", "")
            prompt_text += f"\n{role}: {content}"

        if tools_schema:
            prompt_text += f"\n\nAVAILABLE TOOLS:\n{tools_schema}\n\nTo use a tool, respond with valid JSON: {{'tool': 'tool_name', 'args': {{...}}}}"

        self.token_usage["input"] += self._estimate_tokens(prompt_text)

        loop = asyncio.get_running_loop()

        for attempt in range(max_retries):
            try:
                if self.client_type == "google":
                    # Google GenAI Call
                    def _google_call():
                        response = self.client.models.generate_content(
                            model=self.model,
                            contents=prompt_text
                        )
                        return response.text

                    text = await loop.run_in_executor(None, _google_call)

                elif self.client_type == "openai":
                    # OpenAI Proxy Call
                    def _openai_call():
                        return call_openai_chat(
                            prompt=prompt_text,
                            model=self.model,
                            system=self.system_prompt
                        )
                    text = await loop.run_in_executor(None, _openai_call)

                else:
                    # Dummy Client
                    text = self.client.models.generate_content(None, None).text

                self.token_usage["output"] += self._estimate_tokens(text)
                self.token_usage["total"] = self.token_usage["input"] + self.token_usage["output"]
                return text

            except Exception as e:
                self.log("WARNING", f"LLM call failed (attempt {attempt + 1}/{max_retries}): {e}")
                if attempt < max_retries - 1:
                    sleep_time = backoff_factor ** attempt
                    await asyncio.sleep(sleep_time)
                else:
                    self.log("ERROR", "Max retries reached for LLM call.")
                    raise e
        return ""

    async def think(self, context: str) -> str:
        """
        Internal reasoning step.
        """
        self.log("INFO", "Thinking...")
        messages = [
            {"role": "system", "content": self.system_prompt},
            {"role": "user", "content": f"Context: {context}\n\nThink about the next step. Return a concise plan or reasoning."}
        ]
        response = await self._call_llm(messages)
        self.log("DEBUG", f"Thought: {response}")
        return response

    async def run(self, task: str) -> str:
        """
        Main execution method. Should be overridden by subclasses for specific logic.
        """
        self.log("INFO", f"Starting run for task: {task}")
        start_time = time.time()
        
        try:
            # Default implementation: Think then return thought
            plan = await self.think(task)
            result = plan
            
        except Exception as e:
            self.log("ERROR", f"Run failed: {e}")
            traceback.print_exc()
            result = f"Error: {e}"

        duration = time.time() - start_time
        self.log("INFO", f"Run completed in {duration:.2f}s. Tokens: {self.token_usage['total']}")
        return result

if __name__ == "__main__":
    # Basic verification
    async def main():
        agent = BaseAgent(name="TestAgent")
        print(f"Agent {agent.name} initialized with client type: {agent.client_type}")
        res = await agent.run("Hello world")
        print(f"Result: {res}")

    asyncio.run(main())
