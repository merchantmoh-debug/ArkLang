"""
Router Agent - The orchestrator that analyzes tasks and delegates to specialists.

This agent acts as the "manager" in the swarm, analyzing user requests,
determining which specialist agents to involve, and synthesizing final results.
"""

from typing import Dict, List
from src.agents.base_agent import BaseAgent


class RouterAgent(BaseAgent):
    """
    Router agent responsible for task analysis and delegation.
    
    The Router analyzes incoming tasks, determines which specialist workers
    should handle them, coordinates multi-step workflows, and synthesizes
    the final response from worker outputs.
    """
    
    def __init__(self):
        system_prompt = """You are the Router Agent, the coordinator of a multi-agent system.

Your responsibilities:
1. Analyze user tasks and determine which specialist agents to involve
2. Break down complex tasks into subtasks for different specialists
3. Coordinate the workflow between agents
4. Synthesize final results from multiple specialists

Available specialist agents:
- coder: Writes and refactors code, creates files, implements features
- reviewer: Reviews code quality, checks for security issues, analyzes logs
- researcher: Gathers information, performs web searches, analyzes data

When analyzing a task, respond with a delegation plan in this format:
DELEGATION:
- agent: <agent_name>
- task: <specific task for that agent>

You may delegate to multiple agents in sequence or parallel."""
        
        super().__init__(role="router", system_prompt=system_prompt)
    
    def analyze_and_delegate(self, user_task: str) -> List[Dict[str, str]]:
        """
        Analyze a user task and create a delegation plan.
        
        Args:
            user_task: The task provided by the user.
            
        Returns:
            List of delegation instructions, each containing 'agent' and 'task'.
        """
        analysis = self.execute(user_task)
        
        # Parse the delegation plan from the response
        delegations = []
        lines = analysis.split('\n')
        current_delegation = {}
        
        for line in lines:
            line = line.strip()
            if line.startswith('- agent:'):
                if current_delegation:
                    delegations.append(current_delegation)
                current_delegation = {'agent': line.split(':', 1)[1].strip()}
            elif line.startswith('- task:') and current_delegation:
                current_delegation['task'] = line.split(':', 1)[1].strip()
        
        if current_delegation and 'task' in current_delegation:
            delegations.append(current_delegation)
        
        # Fallback: if no delegations parsed, use simple keyword matching
        if not delegations:
            delegations = self._simple_delegate(user_task)
        
        return delegations
    
    def _simple_delegate(self, task: str) -> List[Dict[str, str]]:
        """
        Simple keyword-based delegation as fallback.
        
        Args:
            task: The task to analyze.
            
        Returns:
            List of delegations based on keywords.
        """
        task_lower = task.lower()
        delegations = []
        
        # Check for code-related keywords
        if any(word in task_lower for word in ['code', 'implement', 'build', 'create', 'write', 'function']):
            delegations.append({'agent': 'coder', 'task': task})
        
        # Check for review-related keywords
        if any(word in task_lower for word in ['review', 'check', 'security', 'quality', 'analyze']):
            delegations.append({'agent': 'reviewer', 'task': task})
        
        # Check for research-related keywords
        if any(word in task_lower for word in ['research', 'search', 'find', 'information', 'learn']):
            delegations.append({'agent': 'researcher', 'task': task})
        
        # Default to coder if no matches
        if not delegations:
            delegations.append({'agent': 'coder', 'task': task})
        
        return delegations
    
    def synthesize_results(self, delegations: List[Dict[str, str]], results: List[str]) -> str:
        """
        Synthesize final response from multiple agent results.
        
        Args:
            delegations: The original delegation plan.
            results: Results from each delegated agent.
            
        Returns:
            Final synthesized response.
        """
        synthesis_prompt = f"""Synthesize a final response based on the following agent outputs:

"""
        for i, (delegation, result) in enumerate(zip(delegations, results), 1):
            synthesis_prompt += f"{i}. [{delegation['agent']}] {delegation['task']}\n"
            synthesis_prompt += f"   Result: {result}\n\n"
        
        synthesis_prompt += "Provide a concise final report summarizing what was accomplished."
        
        return self.execute(synthesis_prompt)
