import sys
import os
import unittest
from unittest.mock import MagicMock

# Add meta directory to path
current_dir = os.path.dirname(os.path.abspath(__file__))
meta_dir = os.path.join(os.path.dirname(current_dir), 'meta')
sys.path.append(meta_dir)

import ark

class TestNetBroadcast(unittest.TestCase):
    def setUp(self):
        # Save original intrinsic
        self.original_send = ark.INTRINSICS.get("sys.net.socket.send")
        self.original_close = ark.INTRINSICS.get("sys.net.socket.close")

    def tearDown(self):
        # Restore original intrinsic
        if self.original_send:
            ark.INTRINSICS["sys.net.socket.send"] = self.original_send
        if self.original_close:
            ark.INTRINSICS["sys.net.socket.close"] = self.original_close

    def test_broadcast_removes_disconnected_peers(self):
        # 1. Define Mock Send
        def mock_send(args):
            handle = args[0].val
            # Fail for handle 2
            if handle == 2:
                return ark.ArkValue(False, "Boolean")
            return ark.ArkValue(True, "Boolean")

        # 2. Define Mock Close (so it doesn't fail)
        def mock_close(args):
            return ark.UNIT_VALUE

        ark.INTRINSICS["sys.net.socket.send"] = mock_send
        ark.INTRINSICS["sys.net.socket.close"] = mock_close

        # 3. Define Ark Script
        # We assume lib/std/net.ark is available relative to repo root
        # We need to ensure the test runs from repo root or handles paths correctly.
        # sys.vm.source uses checks relative to CWD.

        ark_code = """
        sys.vm.source("lib/std/net.ark")

        // Manually inject peers into _net_state (exposed in scope by sys.vm.source)
        _net_state.peers := [
             {socket: 1, ip: "1.1.1.1"},
             {socket: 2, ip: "2.2.2.2"}, // Will fail
             {socket: 3, ip: "3.3.3.3"}
        ]

        net.broadcast("ping")

        // Return count of peers
        len(_net_state.peers)
        """

        # 4. Run
        tree = ark.ARK_PARSER.parse(ark_code)
        scope = ark.Scope()
        scope.set("sys", ark.ArkValue("sys", "Namespace"))
        scope.set("true", ark.ArkValue(1, "Integer"))
        scope.set("false", ark.ArkValue(0, "Integer"))

        # Initialize loaded imports set for import handling
        scope.vars["__loaded_imports__"] = ark.ArkValue(set(), "Set")

        try:
            result = ark.eval_node(tree, scope)
            print(f"Remaining peers: {result.val}")

            # We expect peer 2 to be removed, so count should be 2.
            # If bug is present, count is 3.
            self.assertEqual(result.val, 2, f"Expected 2 peers, got {result.val}. Fix not applied or failed.")

        except Exception as e:
            self.fail(f"Ark Execution Error: {e}")

if __name__ == "__main__":
    unittest.main()
