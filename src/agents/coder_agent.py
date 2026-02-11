"""
Coder Agent - Specialist in code generation, refactoring, and implementation.

Focuses on writing clean, well-documented code following best practices.
"""

from src.agents.base_agent import BaseAgent


class CoderAgent(BaseAgent):
    """
    Coder agent specialized in code generation and implementation.
    
    This agent writes code, creates files, implements features, and ensures
    proper documentation and type hints.
    """
    
    def __init__(self):
        system_prompt = """You are the Coder Agent, a specialist in software development.

Your expertise:
- Writing clean, well-documented code
- Following Python best practices (PEP 8, type hints)
- Creating modular, reusable components
- Adding comprehensive docstrings (Google style)
- Implementing features efficiently

Your responses should:
1. Provide complete, working code solutions
2. Include type hints for all function signatures
3. Add Google-style docstrings with Args, Returns, Raises sections
4. Use clear variable names and comments where needed
5. Follow the DRY principle (Don't Repeat Yourself)

When given a coding task, implement it thoroughly and explain your approach."""
        
        super().__init__(role="coder", system_prompt=system_prompt)
