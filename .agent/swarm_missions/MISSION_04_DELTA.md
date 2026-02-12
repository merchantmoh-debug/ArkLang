# SWARM MISSION: AGENT DELTA (THE GHOST)
**TARGET:** `lib/std/net.ark`
**OBJECTIVE:** Implement Noise Protocol Encryption.

## CONTEXT
Our P2P handshake is plaintext "HELLO".
We need to implement the Noise_XX Handshake Pattern (Mocked for v113.0, but structurally correct).

## INSTRUCTIONS
1.  **Modify `lib/std/net.ark`**:
    - Add `func noise_handshake(socket)` wrapper.
    - Call `sys.crypto.ed25519.gen` (Intrinsic) to get ephemeral keys.
    - Exchange keys.
    - Derive shared secret (Mock this via simple hash(key1 + key2) for now).
2.  **Update `sys_net_socket_send` logic (if possible) or wrap it.**

## CONSTRAINTS
- **DO NOT** break existing plaintext P2P tests yet (Optional: Add `ENABLE_NOISE` flag).
- Focus on the *API Surface* first.
