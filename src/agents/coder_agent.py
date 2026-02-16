import os
import subprocess
import glob
import sys
from typing import Dict, Any, List, Optional
from pathlib import Path

# Ensure project root is in python path
sys.path.append(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from src.agents.base_agent import BaseAgent

# Helper to get project root
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent

class CoderAgent(BaseAgent):
    """
    Coder Agent responsible for writing, modifying, and testing code.
    Has direct access to the filesystem and shell.
    """
    
    def __init__(self, name: str = "Coder", **kwargs):
        system_prompt = (
            "You are the Coder Agent. Your job is to implement features, fix bugs, and write code.\n"
            "You have access to the filesystem and shell.\n"
            "Reference the Ark language syntax and best practices.\n"
            "Output format: JSON with keys 'files_changed' (list), 'tests_added' (list), 'summary' (str)."
        )
        super().__init__(name=name, system_prompt=system_prompt, **kwargs)

        # Register tools
        self.add_tool(self.file_read)
        self.add_tool(self.file_write)
        self.add_tool(self.run_command)
        self.add_tool(self.search_code)
        self.add_tool(self.list_files)

    def _get_path(self, filepath: str) -> Path:
        """Resolve path and ensure it's within project root."""
        try:
            # Check for absolute path attempting traversal
            if os.path.isabs(filepath):
                 filepath = os.path.relpath(filepath, "/")

            path = (PROJECT_ROOT / filepath).resolve()

            # Security check: Ensure path is within project root
            if not str(path).startswith(str(PROJECT_ROOT)):
                 raise ValueError(f"Access denied: Path {filepath} is outside project root.")

            return path
        except Exception as e:
            self.log("ERROR", f"Path resolution error: {e}")
            raise

    def file_read(self, filepath: str) -> str:
        """Read content of a file."""
        try:
            path = self._get_path(filepath)
            if not path.exists():
                return f"Error: File {filepath} does not exist."
            return path.read_text(encoding="utf-8")
        except Exception as e:
            return f"Error reading file {filepath}: {e}"

    def file_write(self, filepath: str, content: str) -> str:
        """Write content to a file. Creates directories if needed."""
        try:
            path = self._get_path(filepath)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
            return f"Successfully wrote to {filepath}"
        except Exception as e:
            return f"Error writing file {filepath}: {e}"

    def list_files(self, directory: str = ".") -> str:
        """List files in a directory."""
        try:
            path = self._get_path(directory)
            if not path.exists():
                 return f"Error: Directory {directory} does not exist."

            files = [str(p.relative_to(PROJECT_ROOT)) for p in path.rglob("*") if p.is_file() and ".git" not in p.parts]
            # Limit output
            if len(files) > 100:
                return "\n".join(files[:100]) + f"\n... ({len(files)-100} more files)"
            return "\n".join(files)
        except Exception as e:
            return f"Error listing files: {e}"

    def run_command(self, command: str) -> str:
        """Run a shell command in the project root."""
        try:
            # Security check: simplistic, but prevents some obvious issues
            # In production, this should be much stricter
            if "rm -rf /" in command:
                return "Error: Command blocked for security."

            result = subprocess.run(
                command,
                shell=True,
                cwd=PROJECT_ROOT,
                capture_output=True,
                text=True,
                timeout=30
            )
            output = f"Exit Code: {result.returncode}\nStdout: {result.stdout}\nStderr: {result.stderr}"
            return output
        except subprocess.TimeoutExpired:
            return "Error: Command timed out."
        except Exception as e:
            return f"Error running command: {e}"

    def search_code(self, pattern: str, path: str = ".") -> str:
        """Search for a pattern in files using grep."""
        try:
            # Ensure path is safe
            target_path = self._get_path(path)

            # Use grep directly, but ensure arguments are quoted properly
            # Using subprocess list form avoids shell injection for arguments
            cmd = ["grep", "-r", pattern, str(target_path)]

            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                cwd=PROJECT_ROOT,
                timeout=30
            )
            return f"Exit Code: {result.returncode}\n{result.stdout}\n{result.stderr}"
        except Exception as e:
            return f"Error searching code: {e}"

    async def run(self, task: str) -> Dict[str, Any]:
        """
        Execute coding task.
        """
        self.log("INFO", f"Starting coding task: {task}")

        # 1. Think and plan
        plan = await self.think(task)

        # 2. Execute (in a real agent loop, this would be iterative)
        # For this framework, we assume the LLM might call tools during 'think' if we implemented
        # a loop inside 'think' or 'run'.

        messages = [
            {"role": "system", "content": self.system_prompt},
            {"role": "user", "content": task}
        ]

        tools_desc = "\n".join([f"{name}: {func.__doc__}" for name, func in self.tools.items()])

        files_changed = []
        tests_added = []
        summary = plan # Default summary is the plan

        max_steps = 5
        for step in range(max_steps):
            response = await self._call_llm(messages, tools_schema=tools_desc)

            # Check for tool call (JSON format expected as per BaseAgent prompt)
            import json
            import re

            tool_call = None
            try:
                # Look for JSON block
                match = re.search(r"\{.*\}", response, re.DOTALL)
                if match:
                    data = json.loads(match.group(0))
                    if "tool" in data:
                        tool_call = data
            except:
                pass

            if tool_call:
                tool_name = tool_call.get("tool")
                args = tool_call.get("args", {})
                self.log("INFO", f"Tool Call: {tool_name} with {args}")

                if tool_name in self.tools:
                    try:
                        func = self.tools[tool_name]
                        # Inspect function signature to pass correct args
                        # For simplicity, pass **args
                        result = func(**args)

                        self.log("INFO", f"Tool Result: {result}")
                        messages.append({"role": "assistant", "content": response})
                        messages.append({"role": "user", "content": f"Tool Output: {result}"})

                        if tool_name == "file_write":
                            filepath = args.get("filepath")
                            if filepath:
                                files_changed.append(filepath)
                                if "test" in filepath:
                                    tests_added.append(filepath)

                    except Exception as e:
                        messages.append({"role": "user", "content": f"Tool Execution Error: {e}"})
                else:
                    messages.append({"role": "user", "content": f"Error: Tool {tool_name} not found."})
            else:
                # No tool call, assume completion
                self.log("INFO", "No tool call detected, assuming completion.")
                summary = response
                break

        return {
            "files_changed": files_changed,
            "tests_added": tests_added,
            "summary": summary
        }

if __name__ == "__main__":
    import asyncio
    async def main():
        coder = CoderAgent()
        print("Testing CoderAgent...")
        # Write a file
        print(coder.file_write("test_output.txt", "Hello Ark"))
        # Read it back
        print(coder.file_read("test_output.txt"))
        # Clean up
        coder.run_command("rm test_output.txt")
        
    asyncio.run(main())
