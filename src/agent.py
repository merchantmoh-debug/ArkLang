import asyncio
import json
import logging
import os
import sys
import time
from datetime import datetime
from typing import Any, Dict, List, Optional, Union
from pathlib import Path

# Ensure project root is in python path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from src.config import settings
from src.agents.base_agent import BaseAgent
from src.agents.router_agent import RouterAgent
from src.agents.coder_agent import CoderAgent
from src.agents.researcher_agent import ResearcherAgent
from src.agents.reviewer_agent import ReviewerAgent

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S"
)
logger = logging.getLogger("Orchestrator")

class AgentOrchestrator:
    """
    Main Orchestrator for the Ark AI Agent Framework.
    Manages task routing, agent instantiation, execution pipeline, and shared memory.
    """

    def __init__(self):
        self.router = RouterAgent()
        self.memory: List[Dict[str, Any]] = []
        self.mcp_manager = None
        self.mcp_tools = []
        
        # Initialize MCP if enabled
        if settings.MCP_ENABLED:
            self._initialize_mcp()

    def _initialize_mcp(self) -> None:
        """Initialize MCP integration and load tools."""
        try:
            from src.mcp_client import MCPClientManagerSync
            from src.tools.mcp_tools import _set_mcp_manager

            logger.info("🔌 Initializing MCP integration...")
            self.mcp_manager = MCPClientManagerSync()
            self.mcp_manager.initialize()
            _set_mcp_manager(self.mcp_manager._async_manager)

            # Get tools as callables
            tools_dict = self.mcp_manager.get_all_tools_as_callables()
            self.mcp_tools = list(tools_dict.values())
            logger.info(f"   🔧 Loaded {len(self.mcp_tools)} MCP tools")

        except ImportError as e:
            logger.warning(f"MCP library not installed: {e}")
        except Exception as e:
            logger.warning(f"Failed to initialize MCP: {e}")

    async def execute_task(self, task: str) -> Dict[str, Any]:
        """
        Execute a high-level task through the agent pipeline.
        Pipeline: Task -> Router -> Specialist -> [Reviewer] -> Result
        """
        logger.info(f"🚀 Starting Orchestrator for task: {task}")
        start_time = time.time()

        # 1. Route Task
        route_decision = await self.router.run(task)
        destination = route_decision.get("destination", "CoderAgent")
        confidence = route_decision.get("confidence", 0.0)

        logger.info(f"📍 Routed to {destination} (Confidence: {confidence})")

        # 2. Instantiate Specialist
        agent: BaseAgent
        if destination == "CoderAgent":
            agent = CoderAgent()
        elif destination == "ResearcherAgent":
            agent = ResearcherAgent()
        elif destination == "ReviewerAgent":
            agent = ReviewerAgent()
        else:
            logger.warning(f"Unknown destination {destination}, defaulting to CoderAgent")
            agent = CoderAgent()

        # Inject MCP tools if available
        for tool in self.mcp_tools:
            try:
                agent.add_tool(tool)
            except Exception as e:
                logger.warning(f"Failed to add MCP tool {tool}: {e}")
        
        # 3. Execute Specialist
        result = await agent.run(task)
        
        # 4. Review Phase (if CoderAgent modified files)
        review_result = None
        if destination == "CoderAgent" and isinstance(result, dict):
            files_changed = result.get("files_changed", [])
            if files_changed:
                logger.info(f"🔍 Files changed: {files_changed}. Initiating review...")
                reviewer = ReviewerAgent()

                review_tasks = []
                for filepath in files_changed:
                    review_tasks.append(reviewer.run(f"Audit {filepath}"))

                if review_tasks:
                    review_results = await asyncio.gather(*review_tasks)
                    # Aggregate
                    issues = []
                    approved = True
                    for res in review_results:
                        issues.extend(res.get("issues", []))
                        if not res.get("approved", False):
                            approved = False

                    review_result = {
                        "issues": issues,
                        "approved": approved
                    }
                    logger.info(f"✅ Review complete. Approved: {approved}, Issues: {len(issues)}")
            else:
                logger.info("No files changed, skipping review.")

        # 5. Compile Final Report
        final_report = {
            "task": task,
            "route": route_decision,
            "execution_result": result,
            "review_result": review_result,
            "duration": time.time() - start_time,
            "token_usage": {
                "router": self.router.token_usage,
                "specialist": agent.token_usage,
                "total": self.router.token_usage["total"] + agent.token_usage["total"]
            }
        }
        
        logger.info("🏁 Task execution complete.")
        return final_report

    def shutdown(self):
        """Cleanup resources."""
        if self.mcp_manager:
            self.mcp_manager.shutdown()

async def main():
    # Allow overriding task via args
    task = " ".join(sys.argv[1:]) or "Write a python script called hello.py that prints 'Hello Ark' and check it for errors."

    orchestrator = AgentOrchestrator()
    try:
        result = await orchestrator.execute_task(task)
        print("\n" + "="*50)
        print(f"FINAL RESULT:\n{json.dumps(result, indent=2, default=str)}")
        print("="*50)
    finally:
        orchestrator.shutdown()

if __name__ == "__main__":
    asyncio.run(main())
