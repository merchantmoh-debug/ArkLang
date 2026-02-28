import asyncio
import json
import logging
import re
import sys
import os
from typing import Dict, Any, List, Optional
from urllib.parse import quote_plus

# Ensure project root is in python path
sys.path.append(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from src.agents.base_agent import BaseAgent

try:
    from playwright.async_api import async_playwright
    PLAYWRIGHT_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_AVAILABLE = False

class ResearcherAgent(BaseAgent):
    """
    Researcher Agent responsible for gathering information from the web.
    Uses Playwright for browser automation.
    """
    
    def __init__(self, name: str = "Researcher", **kwargs):
        system_prompt = (
            "You are the Researcher Agent with expertise in information gathering and analysis.\n"
            "Your job is to find information, analyze data, and summarize findings.\n"
            "You have access to web search and page reading tools.\n"
            "Best Practices: Cross-reference sources, verify claims, cite evidence.\n"
            "Output format: JSON with keys 'findings' (list), 'sources' (list), 'confidence' (0.0-1.0)."
        )
        super().__init__(name=name, system_prompt=system_prompt, **kwargs)

        # Register tools
        self.add_tool(self.web_search)
        self.add_tool(self.read_url)
        self.add_tool(self.summarize)

    async def web_search(self, query: str) -> str:
        """Search the web for a query using DuckDuckGo."""
        if not PLAYWRIGHT_AVAILABLE:
            return "Error: Playwright not available."

        self.log("INFO", f"Searching for: {query}")

        results = []
        try:
            async with async_playwright() as p:
                # Use firefox or chromium
                browser = await p.chromium.launch(headless=True)
                page = await browser.new_page()

                # Use DuckDuckGo HTML version for speed/simplicity
                url = f"https://html.duckduckgo.com/html/?q={quote_plus(query)}"
                await page.goto(url)

                # Extract results
                # DDG HTML structure: .result -> .result__title -> a (href, text)
                # .result__snippet (text)

                # Wait for results to load
                await page.wait_for_selector(".result", timeout=5000)

                # Extract data
                elements = await page.query_selector_all(".result")

                for el in elements[:5]: # Top 5 results
                    title_el = await el.query_selector(".result__title a")
                    snippet_el = await el.query_selector(".result__snippet")

                    if title_el:
                        title = await title_el.inner_text()
                        link = await title_el.get_attribute("href")
                        snippet = await snippet_el.inner_text() if snippet_el else ""

                        results.append(f"Title: {title}\nLink: {link}\nSnippet: {snippet}\n")

                await browser.close()

            if not results:
                return "No results found."

            return "\n".join(results)

        except Exception as e:
            self.log("ERROR", f"Search failed: {e}")
            return f"Error performing search: {e}"

    async def read_url(self, url: str) -> str:
        """Read the content of a specific URL."""
        if not PLAYWRIGHT_AVAILABLE:
            return "Error: Playwright not available."

        self.log("INFO", f"Reading URL: {url}")

        try:
            async with async_playwright() as p:
                browser = await p.chromium.launch(headless=True)
                page = await browser.new_page()

                await page.goto(url, timeout=30000) # 30s timeout

                # Extract visible text
                # Simple extraction: body.innerText
                content = await page.evaluate("document.body.innerText")

                await browser.close()

                # Truncate if too long (e.g. 10k chars)
                if len(content) > 10000:
                     content = content[:10000] + "\n... (truncated)"

                return content

        except Exception as e:
            self.log("ERROR", f"Read URL failed: {e}")
            return f"Error reading URL: {e}"

    async def summarize(self, text: str) -> str:
        """Summarize a long text using the LLM."""
        if len(text) < 500:
            return text # No need to summarize

        context = f"Please summarize the following text efficiently:\n\n{text[:15000]}" # Limit input
        summary = await self.think(context)
        return summary

    async def run(self, task: str) -> Dict[str, Any]:
        """
        Execute research task.
        """
        self.log("INFO", f"Starting research task: {task}")

        messages = [
            {"role": "system", "content": self.system_prompt},
            {"role": "user", "content": task}
        ]

        tools_desc = "\n".join([f"{name}: {func.__doc__}" for name, func in self.tools.items()])

        findings = []
        sources = []
        confidence = 0.5

        max_steps = 5
        for step in range(max_steps):
            response = await self._call_llm(messages, tools_schema=tools_desc)

            # Check for tool call
            tool_call = None
            try:
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
                self.log("INFO", f"Tool Call: {tool_name}")

                if tool_name in self.tools:
                    try:
                        func = self.tools[tool_name]
                        # Handling async tools
                        if asyncio.iscoroutinefunction(func):
                            result = await func(**args)
                        else:
                            result = func(**args)

                        self.log("INFO", f"Tool Result Length: {len(str(result))}")
                        messages.append({"role": "assistant", "content": response})
                        messages.append({"role": "user", "content": f"Tool Output: {result}"})

                        if tool_name == "web_search":
                             # Extract links from result string for sources
                             pass # Ideally parse the result
                        if tool_name == "read_url":
                             sources.append(args.get("url"))
                             findings.append(f"Content from {args.get('url')}")

                    except Exception as e:
                        messages.append({"role": "user", "content": f"Tool Execution Error: {e}"})
                else:
                    messages.append({"role": "user", "content": f"Error: Tool {tool_name} not found."})
            else:
                self.log("INFO", "No tool call detected, assuming completion.")
                # Try to parse findings/sources/confidence from response if structured
                try:
                    data = json.loads(response)
                    if "findings" in data:
                        return data
                except:
                    pass

                # If not structured, wrap it
                return {
                    "findings": [response],
                    "sources": sources,
                    "confidence": confidence
                }

        return {
            "findings": findings,
            "sources": sources,
            "confidence": confidence
        }

if __name__ == "__main__":
    async def main():
        researcher = ResearcherAgent()
        print("Testing ResearcherAgent (mock run without playwright interaction)...")
        # Since running playwright might require installation, we just test instantiation
        # and basic run structure.
        print(f"Playwright Available: {PLAYWRIGHT_AVAILABLE}")
        
    asyncio.run(main())
