"""
Multi-Agent Swarm Orchestration System.

Implements a lightweight Router-Worker pattern for coordinating multiple
specialist agents to solve complex tasks collaboratively.
"""

from typing import Any, Dict, List
from datetime import datetime
from src.agents.router_agent import RouterAgent
from src.agents.coder_agent import CoderAgent
from src.agents.reviewer_agent import ReviewerAgent
from src.agents.researcher_agent import ResearcherAgent


class MessageBus:
    """
    Simple message bus for agent communication.
    
    Maintains a chronological log of all inter-agent messages for context
    sharing and debugging.
    """
    
    def __init__(self):
        """Initialize the message bus with an empty message log."""
        self.messages: List[Dict[str, Any]] = []
    
    def send(self, from_agent: str, to_agent: str, message_type: str, content: str):
        """
        Send a message from one agent to another.
        
        Args:
            from_agent: The sending agent's role.
            to_agent: The receiving agent's role.
            message_type: Type of message (task, result, query).
            content: The message content.
        """
        message = {
            "from": from_agent,
            "to": to_agent,
            "type": message_type,
            "content": content,
            "timestamp": datetime.now().isoformat()
        }
        self.messages.append(message)
    
    def get_context_for(self, agent_name: str) -> List[Dict[str, Any]]:
        """
        Get relevant message context for a specific agent.
        
        Args:
            agent_name: The agent's role name.
            
        Returns:
            List of messages relevant to this agent.
        """
        return [msg for msg in self.messages if msg["to"] == agent_name or msg["from"] == agent_name]
    
    def get_all_messages(self) -> List[Dict[str, Any]]:
        """Get all messages in chronological order."""
        return self.messages.copy()
    
    def clear(self):
        """Clear all messages from the bus."""
        self.messages = []


class SwarmOrchestrator:
    """
    Orchestrates multi-agent collaboration using the Router-Worker pattern.
    
    The orchestrator manages a router agent and multiple specialist workers,
    facilitating task delegation, execution, and result synthesis.
    """
    
    def __init__(self):
        """Initialize the swarm with router and worker agents."""
        print("🪐 Initializing Antigravity Swarm...")
        
        # Initialize message bus
        self.message_bus = MessageBus()
        
        # Initialize router
        print("   🧭 Creating Router agent...")
        self.router = RouterAgent()
        
        # Initialize worker agents
        print("   💻 Creating Coder agent...")
        print("   🔍 Creating Reviewer agent...")
        print("   📚 Creating Researcher agent...")
        self.workers = {
            "coder": CoderAgent(),
            "reviewer": ReviewerAgent(),
            "researcher": ResearcherAgent()
        }
        
        print(f"✅ Swarm initialized with {len(self.workers)} specialist agents!\n")
    
    def execute(self, user_task: str, verbose: bool = True) -> str:
        """
        Execute a user task using the swarm.
        
        Args:
            user_task: The task requested by the user.
            verbose: Whether to print detailed execution logs.
            
        Returns:
            Final synthesized result from the swarm.
        """
        if verbose:
            print(f"🎯 Task Received: {user_task}\n")
            print("=" * 70)
        
        # Step 1: Router analyzes and creates delegation plan
        if verbose:
            print("\n🧭 [Router] Analyzing task and creating delegation plan...")
        
        delegations = self.router.analyze_and_delegate(user_task)
        
        if verbose:
            print(f"   📋 Delegation plan created with {len(delegations)} step(s):")
            for i, delegation in enumerate(delegations, 1):
                print(f"      {i}. {delegation['agent']} → {delegation['task']}")
        
        # Step 2: Execute delegations
        results = []
        for i, delegation in enumerate(delegations, 1):
            agent_name = delegation['agent']
            agent_task = delegation['task']
            
            if verbose:
                print(f"\n{'=' * 70}")
                print(f"📤 [Router → {agent_name.capitalize()}] Delegating task {i}/{len(delegations)}")
                print(f"   Task: {agent_task}")
            
            # Record delegation in message bus
            self.message_bus.send("router", agent_name, "task", agent_task)
            
            # Get worker agent
            worker = self.workers.get(agent_name)
            if not worker:
                result = f"Error: Unknown agent '{agent_name}'"
                results.append(result)
                continue
            
            # Get context for the worker
            context = self.message_bus.get_context_for(agent_name)
            
            # Execute task
            if verbose:
                print(f"\n🔧 [{agent_name.capitalize()}] Executing task...")
            
            result = worker.execute(agent_task, context)
            results.append(result)
            
            # Record result in message bus
            self.message_bus.send(agent_name, "router", "result", result)
            
            if verbose:
                print(f"✅ [{agent_name.capitalize()}] Completed!")
                print(f"   Result preview: {result[:150]}...")
        
        # Step 3: Router synthesizes final result
        if verbose:
            print(f"\n{'=' * 70}")
            print("\n🧭 [Router] Synthesizing final results...")
        
        final_result = self.router.synthesize_results(delegations, results)
        
        if verbose:
            print("\n" + "=" * 70)
            print("🎉 Task Completed!\n")
        
        return final_result
    
    def get_message_log(self) -> List[Dict[str, Any]]:
        """Get the complete message log for debugging."""
        return self.message_bus.get_all_messages()
    
    def reset(self):
        """Reset the swarm state (clear message bus and agent histories)."""
        self.message_bus.clear()
        self.router.reset_history()
        for worker in self.workers.values():
            worker.reset_history()
