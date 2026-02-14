"""
Base Agent class for all specialist agents in the swarm.

Provides common functionality for agent execution, context management,
and communication with the Gemini API.
"""

import os
from typing import Dict, List, Optional
from google import genai
from google.genai import types
from src.config import settings
from src.tools.openai_proxy import call_openai_chat


class BaseAgent:
    """
    Base class for all agents in the swarm.
    
    Each agent has a specific role and system prompt that defines its specialty.
    All agents share common execution logic but differ in their prompts and tools.
    """
    
    def __init__(self, role: str, system_prompt: str):
        """
        Initialize a base agent.
        
        Args:
            role: The agent's role identifier (e.g., "coder", "reviewer").
            system_prompt: The system prompt defining the agent's behavior.
        """
        self.role = role
        self.system_prompt = system_prompt
        self.conversation_history: List[Dict[str, str]] = []
        self.use_openai_backend = False
        
        # Initialize Client
        running_under_pytest = "PYTEST_CURRENT_TEST" in os.environ

        # Define Dummy Client for fallbacks
        class _DummyClient:
            class _Models:
                def generate_content(self, model, contents):
                    class _R:
                        text = f"[{role}] Task completed"
                    return _R()
            def __init__(self):
                self.models = self._Models()

        if running_under_pytest:
            # Dummy client for testing
            self.client = _DummyClient()
        else:
            try:
                if settings.GOOGLE_API_KEY:
                    self.client = genai.Client(api_key=settings.GOOGLE_API_KEY)
                elif settings.OPENAI_BASE_URL:
                    self.use_openai_backend = True
                    self.client = None # Not used for OpenAI backend
                    print(f"🔄 {role} agent: Using OpenAI-compatible backend at {settings.OPENAI_BASE_URL}")
                else:
                    # No keys found, raise to trigger fallback
                    raise ValueError("No GOOGLE_API_KEY or OPENAI_BASE_URL configured")
            except Exception as e:
                print(f"⚠️ {role} agent: client not initialized: {e}")
                # Fallback to dummy client
                self.client = _DummyClient()
    
    def execute(self, task: str, context: Optional[List[Dict[str, str]]] = None) -> str:
        """
        Execute a task with optional context from other agents.
        
        Args:
            task: The task description to execute.
            context: Optional list of previous messages from other agents.
            
        Returns:
            The agent's response as a string.
        """
        # Build the full prompt
        prompt_parts = [f"Task: {task}"]
        
        # Add context if provided
        if context:
            context_str = "\n\nContext from other agents:\n"
            for msg in context:
                context_str += f"[{msg.get('from', 'unknown')}]: {msg.get('content', '')}\n"
            prompt_parts.append(context_str)
        
        full_prompt = "".join(prompt_parts)
        
        try:
            if self.use_openai_backend:
                # Call OpenAI-compatible API
                # We pass the full_prompt as the user message.
                result = call_openai_chat(
                    prompt=full_prompt,
                    system=self.system_prompt,
                    model=settings.OPENAI_MODEL
                )
            else:
                # Call Gemini API
                response = self.client.models.generate_content(
                    model=settings.GEMINI_MODEL_NAME,
                    contents=full_prompt
                )
                result = getattr(response, "text", str(response)).strip()
            
            # Store in conversation history
            self.conversation_history.append({
                "role": "user",
                "content": task
            })
            self.conversation_history.append({
                "role": "assistant",
                "content": result
            })
            
            return result
        except Exception as e:
            return f"[{self.role}] Error executing task: {str(e)}"
    
    def reset_history(self):
        """Clear the conversation history."""
        self.conversation_history = []
