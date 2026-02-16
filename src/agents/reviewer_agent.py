import ast
import json
import re
import sys
import os
from typing import Dict, Any, List, Optional
from pathlib import Path

# Ensure project root is in python path
sys.path.append(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from src.agents.base_agent import BaseAgent

# Helper to get project root
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent

class ReviewerAgent(BaseAgent):
    """
    Reviewer Agent responsible for auditing code quality, security, and correctness.
    """
    
    def __init__(self, name: str = "Reviewer", **kwargs):
        system_prompt = (
            "You are the Reviewer Agent. Your job is to audit code for bugs, security flaws, and style issues.\n"
            "Use the provided tools to analyze the code.\n"
            "Output format: JSON with keys 'issues' (list of {severity, file, line, description, suggestion}) and 'approved' (bool)."
        )
        super().__init__(name=name, system_prompt=system_prompt, **kwargs)

        # Register tools
        self.add_tool(self.check_syntax)
        self.add_tool(self.security_scan)
        self.add_tool(self.read_file)

    def read_file(self, filepath: str) -> str:
        """Read content of a file."""
        try:
            # Handle relative paths properly
            if os.path.isabs(filepath):
                 filepath = os.path.relpath(filepath, "/")
            path = (PROJECT_ROOT / filepath).resolve()
            if not str(path).startswith(str(PROJECT_ROOT)):
                return f"Error: Path {filepath} is outside project root."

            if not path.exists():
                return f"Error: File {filepath} does not exist."
            return path.read_text(encoding="utf-8")
        except Exception as e:
            return f"Error reading file {filepath}: {e}"

    def check_syntax(self, code: str) -> str:
        """Check Python syntax using AST parsing."""
        try:
            ast.parse(code)
            return "Syntax Check: PASSED"
        except SyntaxError as e:
            return f"Syntax Check: FAILED\nLine {e.lineno}: {e.msg}\n{e.text}"
        except Exception as e:
            return f"Syntax Check: ERROR\n{e}"

    def security_scan(self, code: str) -> str:
        """Scan code for common security vulnerabilities (regex-based)."""
        issues = []
        lines = code.split('\n')

        patterns = [
            (r"eval\(", "CRITICAL", "Avoid 'eval()', it allows arbitrary code execution."),
            (r"exec\(", "CRITICAL", "Avoid 'exec()', it allows arbitrary code execution."),
            (r"subprocess\.call\(", "HIGH", "Use 'subprocess.run' with strict arguments instead of 'call'."),
            (r"shell=True", "HIGH", "Avoid 'shell=True' in subprocess calls to prevent injection."),
            (r"pickle\.load", "HIGH", "Avoid 'pickle.load' on untrusted data."),
            (r"input\(", "MEDIUM", "Avoid 'input()' in production code."),
            (r"print\(", "LOW", "Use logging instead of 'print()'."),
            (r"except Exception:", "LOW", "Avoid catching generic Exception without logging."),
            (r"pass", "INFO", "Empty 'pass' block found.")
        ]

        for i, line in enumerate(lines):
            for pattern, severity, desc in patterns:
                if re.search(pattern, line):
                    issues.append(f"Line {i+1} [{severity}]: {desc}")

        if not issues:
            return "Security Scan: PASSED (No obvious issues found)"

        return "Security Scan: ISSUES FOUND\n" + "\n".join(issues)

    async def run(self, task: str) -> Dict[str, Any]:
        """
        Execute review task.
        """
        self.log("INFO", f"Starting review task: {task}")

        messages = [
            {"role": "system", "content": self.system_prompt},
            {"role": "user", "content": task}
        ]

        tools_desc = "\n".join([f"{name}: {func.__doc__}" for name, func in self.tools.items()])

        review_result = {
            "issues": [],
            "approved": False
        }

        # 1. Analyze using tools (if file provided)
        # Check if task mentions a file (case-insensitive)
        file_match = re.search(r"file:? (\S+)", task, re.IGNORECASE) or re.search(r"audit (\S+\.py)", task, re.IGNORECASE)
        target_file = None
        code_content = None

        if file_match:
            target_file = file_match.group(1)
            self.log("INFO", f"Identified target file: {target_file}")
            code_content = self.read_file(target_file)

            if "Error" not in code_content:
                # Run syntax check
                syntax_res = self.check_syntax(code_content)
                self.log("INFO", syntax_res)
                messages.append({"role": "user", "content": f"Syntax Check Result: {syntax_res}"})

                if "FAILED" in syntax_res:
                    review_result["issues"].append({
                        "severity": "CRITICAL",
                        "file": target_file,
                        "line": 0,
                        "description": "Syntax Error",
                        "suggestion": "Fix syntax error."
                    })
                    review_result["approved"] = False
                    return review_result

                # Run security scan
                sec_res = self.security_scan(code_content)
                self.log("INFO", sec_res)
                messages.append({"role": "user", "content": f"Security Scan Result: {sec_res}"})

                if "ISSUES FOUND" in sec_res:
                    # Parse security issues
                    for line in sec_res.split('\n')[1:]:
                         match = re.search(r"Line (\d+) \[(.*?)\]: (.*)", line)
                         if match:
                             review_result["issues"].append({
                                 "severity": match.group(2),
                                 "file": target_file,
                                 "line": int(match.group(1)),
                                 "description": match.group(3),
                                 "suggestion": "See description."
                             })
            else:
                 self.log("ERROR", f"Could not read file: {code_content}")
                 messages.append({"role": "user", "content": f"Error reading file: {code_content}"})

        # 2. Ask LLM for comprehensive review
        response = await self._call_llm(messages, tools_schema=tools_desc)

        # 3. Parse LLM response
        try:
            match = re.search(r"\{.*\}", response, re.DOTALL)
            if match:
                data = json.loads(match.group(0))
                if "issues" in data:
                    # Merge LLM issues with tool issues
                    review_result["issues"].extend(data["issues"])
                    review_result["approved"] = data.get("approved", False)
                    return review_result
        except:
            pass

        # If parsing failed, fallback
        if not review_result["issues"] and "approved" in response.lower():
             review_result["approved"] = True

        # If security scan had issues, force disapproval unless overridden (unlikely)
        if any(i["severity"] in ["CRITICAL", "HIGH"] for i in review_result["issues"]):
            review_result["approved"] = False

        return review_result

if __name__ == "__main__":
    import asyncio
    async def main():
        reviewer = ReviewerAgent()
        print("Testing ReviewerAgent...")

        # Create a dummy file with issues
        dummy_file = "vulnerable.py"
        with open(dummy_file, "w") as f:
            f.write("import os\nval = eval(input('Enter code: '))\nprint(val)")

        res = await reviewer.run(f"Audit {dummy_file}")
        print(json.dumps(res, indent=2))

        os.remove(dummy_file)
        
    asyncio.run(main())
