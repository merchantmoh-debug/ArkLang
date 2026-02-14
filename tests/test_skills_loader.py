import pytest
from pathlib import Path
from typing import Dict, Any, Callable
from src.skills.loader import load_skills

def test_load_skills_empty(tmp_path):
    """Test loading from an empty directory."""
    agent_tools: Dict[str, Callable[..., Any]] = {}
    skills_dir = tmp_path / "skills"
    skills_dir.mkdir()

    docs = load_skills(agent_tools, skills_dir=skills_dir)

    assert docs == ""
    assert len(agent_tools) == 0

def test_load_skills_valid_skill(tmp_path):
    """Test loading a valid skill with tools and documentation."""
    agent_tools: Dict[str, Callable[..., Any]] = {}
    skills_dir = tmp_path / "skills"
    skills_dir.mkdir()

    # Create a skill directory
    my_skill = skills_dir / "my_skill"
    my_skill.mkdir()

    # Create tools.py
    tools_content = """
def my_tool(x: int) -> int:
    '''A dummy tool.'''
    return x + 1

def _private_helper():
    pass
"""
    (my_skill / "tools.py").write_text(tools_content, encoding="utf-8")

    # Create SKILL.md
    doc_content = "This is the documentation for my skill."
    (my_skill / "SKILL.md").write_text(doc_content, encoding="utf-8")

    docs = load_skills(agent_tools, skills_dir=skills_dir)

    # Verify tool loaded
    assert "my_tool" in agent_tools
    assert agent_tools["my_tool"](5) == 6
    assert "_private_helper" not in agent_tools

    # Verify docs loaded
    assert "--- SKILL: my_skill ---" in docs
    assert doc_content in docs

def test_load_skills_ignore_private(tmp_path):
    """Test that private directories (starting with _) are ignored."""
    agent_tools: Dict[str, Callable[..., Any]] = {}
    skills_dir = tmp_path / "skills"
    skills_dir.mkdir()

    # Create a private skill directory
    private_skill = skills_dir / "_private_skill"
    private_skill.mkdir()

    (private_skill / "tools.py").write_text("def secret_tool(): pass", encoding="utf-8")
    (private_skill / "SKILL.md").write_text("Secret docs", encoding="utf-8")

    docs = load_skills(agent_tools, skills_dir=skills_dir)

    assert docs == ""
    assert len(agent_tools) == 0

def test_load_skills_broken_tool(tmp_path, capsys):
    """Test that a syntax error in tools.py is handled gracefully."""
    agent_tools: Dict[str, Callable[..., Any]] = {}
    skills_dir = tmp_path / "skills"
    skills_dir.mkdir()

    broken_skill = skills_dir / "broken_skill"
    broken_skill.mkdir()

    # Invalid Python syntax
    (broken_skill / "tools.py").write_text("def broken_tool() return 1", encoding="utf-8")

    docs = load_skills(agent_tools, skills_dir=skills_dir)

    # Check that it didn't crash and logged an error
    captured = capsys.readouterr()
    assert "Failed to load tools" in captured.out
    assert len(agent_tools) == 0

def test_load_skills_no_files(tmp_path):
    """Test a skill directory that has no tools.py or SKILL.md."""
    agent_tools: Dict[str, Callable[..., Any]] = {}
    skills_dir = tmp_path / "skills"
    skills_dir.mkdir()

    empty_skill = skills_dir / "empty_skill"
    empty_skill.mkdir()

    docs = load_skills(agent_tools, skills_dir=skills_dir)

    assert docs == ""
    assert len(agent_tools) == 0

def test_load_skills_missing_dir(tmp_path, capsys):
    """Test when the skills directory itself does not exist."""
    agent_tools: Dict[str, Callable[..., Any]] = {}
    skills_dir = tmp_path / "non_existent_skills"

    docs = load_skills(agent_tools, skills_dir=skills_dir)

    captured = capsys.readouterr()
    assert "Skills directory not found" in captured.out
    assert docs == ""
