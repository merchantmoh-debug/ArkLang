import unittest
import struct
from meta.bytecode import BytecodeEmitter, BytecodeDisassembler, OPCODES

class TestBytecode(unittest.TestCase):
    def test_constant_pool(self):
        emitter = BytecodeEmitter()
        idx1 = emitter.get_const_index(42, 'Integer')
        idx2 = emitter.get_const_index(42, 'Integer')
        idx3 = emitter.get_const_index(3.14, 'Float')
        idx4 = emitter.get_const_index("hello", 'String')

        self.assertEqual(idx1, idx2) # Deduplication
        self.assertNotEqual(idx1, idx3)
        self.assertEqual(len(emitter.constants), 3)
        self.assertEqual(emitter.constants[0], (42, 'Integer'))
        self.assertEqual(emitter.constants[1], (3.14, 'Float'))
        self.assertEqual(emitter.constants[2], ("hello", 'String'))

    def test_emit_push_add_print(self):
        emitter = BytecodeEmitter()

        # push 10
        idx1 = emitter.get_const_index(10, 'Integer')
        emitter.emit('PUSH_CONST', idx1)

        # push 20
        idx2 = emitter.get_const_index(20, 'Integer')
        emitter.emit('PUSH_CONST', idx2)

        # add
        emitter.emit('ADD')

        # print
        emitter.emit('PRINT')

        # Check code bytes
        # PUSH_CONST(1) + u16(0)
        # PUSH_CONST(1) + u16(1)
        # ADD(4)
        # PRINT(15)

        expected = bytearray()
        expected.append(OPCODES['PUSH_CONST'])
        expected.extend(struct.pack('>H', 0))
        expected.append(OPCODES['PUSH_CONST'])
        expected.extend(struct.pack('>H', 1))
        expected.append(OPCODES['ADD'])
        expected.append(OPCODES['PRINT'])

        self.assertEqual(emitter.code, expected)

    def test_roundtrip_disasm(self):
        emitter = BytecodeEmitter()

        # x := 10 + 5
        # print(x)

        # Consts: 10, 5, "x" (var names not in const pool usually, but in instructions)
        c10 = emitter.get_const_index(10, 'Integer')
        c5 = emitter.get_const_index(5, 'Integer')
        vx = emitter.get_var_index('x')

        emitter.emit('PUSH_CONST', c10)
        emitter.emit('PUSH_CONST', c5)
        emitter.emit('ADD')
        emitter.emit('STORE_VAR', vx)
        emitter.emit('LOAD_VAR', vx)
        emitter.emit('PRINT')

        bytecode = emitter.to_bytes()

        disassembler = BytecodeDisassembler()
        text = disassembler.disassemble(bytecode)

        self.assertIn("Constant Pool:", text)
        self.assertIn("Int(10)", text)
        self.assertIn("Int(5)", text)
        self.assertIn("PUSH_CONST", text)
        self.assertIn("ADD", text)
        self.assertIn("STORE_VAR", text)
        self.assertIn("PRINT", text)

        # Verify header magic
        self.assertTrue(bytecode.startswith(b'ARKB\x01'))

    def test_header_magic(self):
        emitter = BytecodeEmitter()
        bytecode = emitter.to_bytes()
        self.assertEqual(bytecode[:4], b'ARKB')
        self.assertEqual(bytecode[4], 0x01)

if __name__ == '__main__':
    unittest.main()
