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


class MessageBus:
    """
    Inter-agent message bus for swarm communication.
    Stores messages as dicts with from, to, type, content, timestamp fields.
    """

    def __init__(self):
        self.messages: list = []

    def send(self, sender: str, recipient: str, msg_type: str, content: str):
        """Send a message between agents."""
        self.messages.append({
            "from": sender,
            "to": recipient,
            "type": msg_type,
            "content": content,
            "timestamp": datetime.now().isoformat(),
        })

    def get_all_messages(self) -> list:
        """Return a copy of all messages."""
        return list(self.messages)

    def get_context_for(self, agent_name: str) -> list:
        """Get messages relevant to a specific agent (sent by or to them)."""
        return [
            m for m in self.messages
            if m["from"] == agent_name or m["to"] == agent_name
        ]

    def clear(self):
        """Clear all messages."""
        self.messages.clear()

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

    def get_message_log(self):
        """Return a copy of the message bus."""
        return list(self.message_bus)

    def execute(self, task: str, strategy: str = "router", verbose: bool = True) -> str:
        """Execute a task with the specified strategy (synchronous)."""
        start_time = datetime.now()
        try:
            if strategy == "router":
                result = self._strategy_router_sync(task, verbose=verbose)
            elif strategy == "broadcast":
                result = self._strategy_broadcast_sync(task)
            elif strategy == "consensus":
                result = self._strategy_consensus_sync(task)
            else:
                raise ValueError(f"Unknown strategy: {strategy}")

            latency = (datetime.now() - start_time).total_seconds()
            self._update_stats(success=True, latency=latency)
            return result

        except Exception as e:
            latency = (datetime.now() - start_time).total_seconds()
            self._update_stats(success=False, latency=latency)
            return f"Error: {e}"

    async def _run_sync(self, func, *args):
        """Run a synchronous function in the executor."""
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(self._executor, func, *args)

    def _strategy_router_sync(self, task: str, verbose: bool = True) -> str:
        """Synchronous router strategy for test compatibility."""
        router = self.agents.get("router")
        if not router:
            return "Error: Router agent not found."

        # Analyze
        if hasattr(router, "analyze_and_delegate"):
            delegations = router.analyze_and_delegate(task)
        else:
            delegations = [{"agent": "coder", "task": task}]

        results = []

        if isinstance(delegations, list):
            for item in delegations:
                if isinstance(item, dict):
                    agent_name = item.get("agent")
                    subtask = item.get("task")

                    # Log task to message bus
                    self.message_bus.append({
                        "from": "router",
                        "to": agent_name,
                        "type": "task",
                        "content": subtask,
                        "timestamp": datetime.now().isoformat(),
                    })

                    if agent_name in self.agents:
                        agent = self.agents[agent_name]
                        res = agent.execute(subtask)
                        results.append(res)

                        # Log result to message bus
                        self.message_bus.append({
                            "from": agent_name,
                            "to": "router",
                            "type": "result",
                            "content": res,
                            "timestamp": datetime.now().isoformat(),
                        })
                    else:
                        error_msg = f"Error: Unknown agent '{agent_name}'"
                        results.append(error_msg)

        # Synthesize
        if hasattr(router, "synthesize_results"):
            return router.synthesize_results(delegations, results)
        else:
            return "\n".join([str(r) for r in results])

    def _strategy_broadcast_sync(self, task: str) -> str:
        """Synchronous broadcast strategy."""
        results = {}
        for name, agent in self.agents.items():
            if name != "router":
                results[name] = agent.execute(task)
        return str(results)

    def _strategy_consensus_sync(self, task: str) -> str:
        """Synchronous consensus strategy."""
        results = {}
        for name, agent in self.agents.items():
            if name != "router":
                results[name] = agent.execute(task)
        return str(results)

    async def _strategy_router(self, task: str) -> Any:
        """Use RouterAgent to delegate (async version)."""
        return self._strategy_router_sync(task)

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
