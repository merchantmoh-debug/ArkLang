import unittest
from src.agents.coder_agent import CoderAgent

class TestCoderAgentSecurity(unittest.TestCase):
    def setUp(self):
        self.agent = CoderAgent()

    def test_basic_command(self):
        """Test a safe command passes."""
        # 'ls' is generally safe in this context
        result = self.agent.run_command("ls")
        self.assertIn("Exit Code: 0", result)
        self.assertNotIn("Error: Command blocked", result)

    def test_semicolon_injection(self):
        """Test semicolon injection is blocked."""
        result = self.agent.run_command("ls; echo INJECTED")
        self.assertTrue("Error: Command blocked" in result or "INJECTED" not in result)
        # For the purpose of this task, we want it to be explicitly blocked
        self.assertIn("Error: Command blocked", result)

    def test_ampersand_injection(self):
        """Test ampersand injection is blocked."""
        result = self.agent.run_command("ls && echo INJECTED")
        self.assertIn("Error: Command blocked", result)

    def test_pipe_injection(self):
        """Test pipe injection is blocked."""
        result = self.agent.run_command("ls | grep src")
        self.assertIn("Error: Command blocked", result)

    def test_backtick_injection(self):
        """Test backtick injection is blocked."""
        result = self.agent.run_command("echo `whoami` ")
        self.assertIn("Error: Command blocked", result)

    def test_subshell_injection(self):
        """Test $(...) injection is blocked."""
        result = self.agent.run_command("echo $(whoami)")
        self.assertIn("Error: Command blocked", result)

    def test_redirection_injection(self):
        """Test redirection is blocked."""
        result = self.agent.run_command("echo 'owned' > /tmp/owned")
        self.assertIn("Error: Command blocked", result)

    def test_rm_rf_bypass_space(self):
        """Test rm -rf / bypass with extra space."""
        result = self.agent.run_command("rm  -rf /")
        self.assertIn("Error: Command blocked", result)

    def test_rm_rf_etc(self):
        """Test rm -rf /etc is blocked."""
        result = self.agent.run_command("rm -rf /etc")
        self.assertIn("Error: Command blocked", result)

if __name__ == "__main__":
    unittest.main()
