import json
import re
import sys
import os
from typing import Dict, Any, Optional

# Ensure project root is in python path
sys.path.append(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from src.agents.base_agent import BaseAgent

class RouterAgent(BaseAgent):
    """
    Router Agent responsible for analyzing tasks and directing them
    to the appropriate specialist agent.
    """
    
    def __init__(self, name: str = "Router", **kwargs):
        system_prompt = (
            "You are the Router Agent. Your job is to analyze the user's request and route it "
            "to the most appropriate specialist agent.\n"
            "Specialists:\n"
            "- CoderAgent: Writes code, implements features, fixes bugs.\n"
            "- ResearcherAgent: Searches the web, gathers information, analyzes data.\n"
            "- ReviewerAgent: Reviews code, audits security, checks quality.\n\n"
            "Routing Logic:\n"
            "- 'write code', 'implement', 'fix bug' -> CoderAgent\n"
            "- 'research', 'find', 'search', 'analyze' -> ResearcherAgent\n"
            "- 'review', 'audit', 'check' -> ReviewerAgent\n"
            "- Default -> CoderAgent\n\n"
            "Output a JSON object with keys: 'destination', 'confidence' (0.0-1.0), 'reasoning'."
        )
        super().__init__(name=name, system_prompt=system_prompt, **kwargs)

    async def run(self, task: str) -> Dict[str, Any]:
        """
        Analyze the task and return routing decision.
        """
        self.log("INFO", f"Analyzing task: {task}")
        
        # 1. Ask LLM for decision
        context = f"Task: {task}"
        response_text = await self.think(context)
        
        decision = self._parse_response(response_text)
        
        # 2. Fallback to heuristic if LLM failed or returned invalid JSON (e.g. DummyClient)
        if not decision:
            self.log("WARNING", "LLM response parsing failed or empty. Using heuristics.")
            decision = self._apply_heuristics(task)
            
        self.log("INFO", f"Routing decision: {decision.get('destination')} (Confidence: {decision.get('confidence')})")
        return decision

    def _parse_response(self, text: str) -> Optional[Dict[str, Any]]:
        """Extract JSON from LLM response."""
        try:
            # Attempt to find JSON block
            match = re.search(r"\{.*\}", text, re.DOTALL)
            if match:
                data = json.loads(match.group(0))
                if "destination" in data:
                    return data
        except Exception as e:
            self.log("DEBUG", f"JSON parsing failed: {e}")
        return None

    def _apply_heuristics(self, task: str) -> Dict[str, Any]:
        """Apply keyword-based routing rules."""
        task_lower = task.lower()
        
        # Heuristics based on keyword presence
        # Prioritize specifically requested actions
        
        if any(kw in task_lower for kw in ["review", "audit", "check code", "review code"]):
            return {"destination": "ReviewerAgent", "confidence": 0.9, "reasoning": "Keyword match: review/audit"}
        
        if any(kw in task_lower for kw in ["research", "find", "search", "analyze", "look up"]):
            return {"destination": "ResearcherAgent", "confidence": 0.9, "reasoning": "Keyword match: research/search"}
            
        if any(kw in task_lower for kw in ["write code", "implement", "fix bug", "create function", "script", "program"]):
            return {"destination": "CoderAgent", "confidence": 0.9, "reasoning": "Keyword match: coding terms"}

        return {"destination": "CoderAgent", "confidence": 0.5, "reasoning": "Default fallback"}

if __name__ == "__main__":
    import asyncio
    async def main():
        router = RouterAgent()
        print("Testing RouterAgent...")
        # Test 1: Code
        res1 = await router.run("Write a python script to calculate pi")
        print(f"Task 1 Result: {res1}")
        # Test 2: Research
        res2 = await router.run("Search for the latest Ark Core version")
        print(f"Task 2 Result: {res2}")
        # Test 3: Review
        res3 = await router.run("Audit this code for security flaws")
        print(f"Task 3 Result: {res3}")
        
    asyncio.run(main())
