import asyncio
import collections
from datetime import datetime
from typing import Any, Dict, List, Optional, Union
from concurrent.futures import ThreadPoolExecutor

# Try to import agents, fallback to mocks if not available (for robust testing)
try:
    from src.agents.router_agent import RouterAgent
    from src.agents.coder_agent import CoderAgent
    from src.agents.reviewer_agent import ReviewerAgent
    from src.agents.researcher_agent import ResearcherAgent
except ImportError:
    class BaseAgent:
        def __init__(self, role="mock"): self.role = role
        def execute(self, task, context=None): return f"[{self.role}] Executed: {task}"
        def analyze_and_delegate(self, task): return [{"agent": "coder", "task": task}]
        def synthesize_results(self, delegations, results): return "\n".join(results)
    
    RouterAgent = lambda: BaseAgent("router")
    CoderAgent = lambda: BaseAgent("coder")
    ReviewerAgent = lambda: BaseAgent("reviewer")
    ResearcherAgent = lambda: BaseAgent("researcher")

from src.config import settings

class SwarmOrchestrator:
    """
    Swarm Orchestrator.
    Manages multiple agents and executes tasks using various strategies.
    """

    def __init__(self):
        self.message_bus = [] # Simple list for now
        self.agents = {
            "router": RouterAgent(),
            "coder": CoderAgent(),
            "reviewer": ReviewerAgent(),
            "researcher": ResearcherAgent()
        }
        self.stats = {
            "tasks_completed": 0,
            "tasks_failed": 0,
            "total_tokens_used": 0,
            "average_latency": 0.0
        }
        self._executor = ThreadPoolExecutor(max_workers=4)

    def add_agent(self, agent):
        """Register an agent in the swarm."""
        self.agents[agent.role] = agent

    async def execute(self, task: str, strategy: str = "router") -> Dict[str, Any]:
        """Execute a task with the specified strategy."""
        start_time = datetime.now()
        success = False
        try:
            if strategy == "router":
                result = await self._strategy_router(task)
            elif strategy == "broadcast":
                result = await self._strategy_broadcast(task)
            elif strategy == "consensus":
                result = await self._strategy_consensus(task)
            else:
                raise ValueError(f"Unknown strategy: {strategy}")

            success = True
            latency = (datetime.now() - start_time).total_seconds()
            self._update_stats(success=True, latency=latency)

            if isinstance(result, str):
                return {"result": result, "status": "success"}
            return result

        except Exception as e:
            latency = (datetime.now() - start_time).total_seconds()
            self._update_stats(success=False, latency=latency)
            return {"result": str(e), "status": "error"}

    async def _run_sync(self, func, *args):
        """Run a synchronous function in the executor."""
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(self._executor, func, *args)

    async def _strategy_router(self, task: str) -> Any:
        """Use RouterAgent to delegate."""
        router = self.agents.get("router")
        if not router:
            return "Error: Router agent not found."

        # Analyze
        if hasattr(router, "analyze_and_delegate"):
            delegations = await self._run_sync(router.analyze_and_delegate, task)
        else:
            delegations = [{"agent": "coder", "task": task}] # Fallback
        
        results = []
        delegation_plan = []
        
        # If delegation returns list of dicts
        if isinstance(delegations, list):
            for item in delegations:
                if isinstance(item, dict):
                    agent_name = item.get("agent")
                    subtask = item.get("task")
                    delegation_plan.append(item)

                    if agent_name in self.agents:
                        agent = self.agents[agent_name]
                        res = await self._run_sync(agent.execute, subtask)
                        results.append(res)
                    else:
                        results.append(f"Error: Agent {agent_name} not found.")
        
        # Synthesize
        if hasattr(router, "synthesize_results"):
             final_res = await self._run_sync(router.synthesize_results, delegation_plan, results)
        else:
             final_res = "\n".join([str(r) for r in results])

        return final_res

    async def _strategy_broadcast(self, task: str) -> Dict[str, Any]:
        """Send task to all agents (except router)."""
        futures = []
        agent_names = []
        for name, agent in self.agents.items():
            if name == "router": continue
            agent_names.append(name)
            futures.append(self._run_sync(agent.execute, task))
        
        if not futures:
            return {"error": "No worker agents available."}

        results = await asyncio.gather(*futures)
        return {name: res for name, res in zip(agent_names, results)}

    async def _strategy_consensus(self, task: str) -> Dict[str, Any]:
        """Send to available agents and return results."""
        target_agents = ["coder", "reviewer", "researcher"]
        futures = []
        used_agents = []
        
        for name in target_agents:
            if name in self.agents:
                used_agents.append(name)
                futures.append(self._run_sync(self.agents[name].execute, task))
        
        if not futures:
             return {"error": "No consensus agents available."}

        results = await asyncio.gather(*futures)
        return {"consensus_results": {k: v for k, v in zip(used_agents, results)}}

    async def execute_parallel(self, tasks: List[str]) -> List[Any]:
        """Run multiple tasks in parallel using the default strategy (router)."""
        futures = [self.execute(t, strategy="router") for t in tasks]
        return await asyncio.gather(*futures)

    async def execute_pipeline(self, task: str, pipeline: List[str]) -> Dict[str, Any]:
        """Sequential agent chain."""
        current_result = task
        history = []

        for agent_name in pipeline:
            if agent_name not in self.agents:
                return {"error": f"Agent {agent_name} not found"}
            
            agent = self.agents[agent_name]
            # Pass previous result as task
            res = await self._run_sync(agent.execute, current_result)
            
            # Update current_result for next step if res is string
            # If res is complex, we might need logic here. Assuming string flow.
            current_result = str(res)
            history.append({agent_name: current_result})
            
        return {"result": current_result, "pipeline_history": history}

    def _update_stats(self, success: bool, latency: float):
        if success:
            self.stats["tasks_completed"] += 1
        else:
            self.stats["tasks_failed"] += 1
        
        n = self.stats["tasks_completed"] + self.stats["tasks_failed"]
        prev_avg = self.stats["average_latency"]
        # Update moving average
        if n > 0:
            self.stats["average_latency"] = prev_avg + (latency - prev_avg) / n

    def status(self) -> Dict[str, Any]:
        """Return swarm health metrics."""
        return self.stats

    def report(self) -> str:
        """Formatted status report."""
        s = self.stats
        return (
            f"Swarm Status Report:\n"
            f"  Completed: {s['tasks_completed']}\n"
            f"  Failed:    {s['tasks_failed']}\n"
            f"  Avg Latency: {s['average_latency']:.2f}s"
        )
