import struct

OPCODES = {
    'PUSH_CONST': 0x01,
    'LOAD_VAR': 0x02,
    'STORE_VAR': 0x03,
    'ADD': 0x04,
    'SUB': 0x05,
    'MUL': 0x06,
    'DIV': 0x07,
    'CMP_EQ': 0x08,
    'CMP_LT': 0x09,
    'CMP_GT': 0x0A,
    'JUMP': 0x0B,
    'JUMP_IF_FALSE': 0x0C,
    'CALL': 0x0D,
    'RETURN': 0x0E,
    'PRINT': 0x0F,
    'HALT': 0x10,
    'POP': 0x11,
    'DUP': 0x12,
    'NEG': 0x13,
    'NOT': 0x14,
    'MOD': 0x15,
}

OPCODE_NAMES = {v: k for k, v in OPCODES.items()}

class BytecodeEmitter:
    def __init__(self):
        self.constants = []
        self.code = bytearray()
        self.var_map = {} # name -> index
        self.var_count = 0
        self.labels = {} # label_name -> address
        self.jumps_to_patch = [] # (address_of_arg, label_name)
        self.function_labels = {} # func_name -> label_name

    def get_const_index(self, val, type_str):
        # Deduplicate constants
        for i, (c_val, c_type) in enumerate(self.constants):
            if c_val == val and c_type == type_str:
                return i
        self.constants.append((val, type_str))
        return len(self.constants) - 1

    def get_var_index(self, name):
        if name not in self.var_map:
            if self.var_count >= 65536:
                raise Exception("Too many variables (limit 65536)")
            self.var_map[name] = self.var_count
            self.var_count += 1
        return self.var_map[name]

    def emit(self, opcode_name, *operands):
        if opcode_name not in OPCODES:
            raise ValueError(f"Unknown opcode: {opcode_name}")
        opcode = OPCODES[opcode_name]
        self.code.append(opcode)

        for i, op in enumerate(operands):
            if isinstance(op, int):
                # Emit u16 operand by default unless specified otherwise
                # CALL arg_count is u8 (second operand)
                if opcode_name == 'CALL' and i == 1:
                     self.code.append(op & 0xFF)
                else:
                    self.code.extend(struct.pack('>H', op))
            elif isinstance(op, str):
                # Label for jump target
                # Reserve 2 bytes for u16 address and record patch location
                self.jumps_to_patch.append((len(self.code), op))
                self.code.extend(b'\x00\x00')
            else:
                raise ValueError(f"Invalid operand type: {type(op)}")

    def define_label(self, name):
        self.labels[name] = len(self.code)

    def _patch_jumps(self):
        for addr, label in self.jumps_to_patch:
            if label not in self.labels:
                raise Exception(f"Undefined label: {label}")
            target = self.labels[label]
            struct.pack_into('>H', self.code, addr, target)

    def to_bytes(self):
        self._patch_jumps()

        # Header: ARKB + version 0x01 + 3 reserved bytes
        header = b'ARKB\x01\x00\x00\x00'

        # Constant Pool
        cp_bytes = bytearray()
        cp_bytes.extend(struct.pack('>H', len(self.constants)))
        for val, type_str in self.constants:
            if type_str == 'Integer':
                cp_bytes.append(0x01)
                cp_bytes.extend(struct.pack('>q', val)) # 8 bytes signed
            elif type_str == 'String':
                cp_bytes.append(0x02)
                encoded = val.encode('utf-8')
                cp_bytes.extend(struct.pack('>H', len(encoded)))
                cp_bytes.extend(encoded)
            elif type_str == 'Boolean':
                cp_bytes.append(0x03)
                cp_bytes.append(1 if val else 0)
            elif type_str == 'Float': # Assuming float supported
                cp_bytes.append(0x04)
                cp_bytes.extend(struct.pack('>d', float(val)))
            else:
                raise Exception(f"Unsupported constant type: {type_str}")

        return header + cp_bytes + self.code

    def compile_ast(self, tree):
        self._visit(tree)
        return self.to_bytes()

    def _visit(self, node):
        if hasattr(node, 'data'):
            method_name = f"_visit_{node.data}"
            visitor = getattr(self, method_name, self._generic_visit)
            return visitor(node)
        return None # Leaf nodes handled in parent or generic

    def _generic_visit(self, node):
        if hasattr(node, 'children'):
            for child in node.children:
                self._visit(child)

    # --- Visitors ---

    def _visit_start(self, node):
        for child in node.children:
            self._visit(child)
        self.emit('HALT')

    def _visit_block(self, node):
        for child in node.children:
            self._visit(child)

    def _visit_number(self, node):
        val = int(node.children[0].value)
        idx = self.get_const_index(val, 'Integer')
        self.emit('PUSH_CONST', idx)

    def _visit_string(self, node):
        # Remove quotes
        try:
            val = node.children[0].value[1:-1]
            # Handle escapes if necessary?
            import ast
            try:
                val = ast.literal_eval(node.children[0].value)
            except:
                pass
        except:
            val = ""
        idx = self.get_const_index(val, 'String')
        self.emit('PUSH_CONST', idx)

    def _visit_var(self, node):
        name = node.children[0].value
        # If 'true' or 'false', treat as const?
        if name == 'true':
            idx = self.get_const_index(True, 'Boolean')
            self.emit('PUSH_CONST', idx)
            return
        if name == 'false':
            idx = self.get_const_index(False, 'Boolean')
            self.emit('PUSH_CONST', idx)
            return

        idx = self.get_var_index(name)
        self.emit('LOAD_VAR', idx)

    def _visit_assign_var(self, node):
        name = node.children[0].value
        expr = node.children[1]
        self._visit(expr)
        idx = self.get_var_index(name)
        self.emit('STORE_VAR', idx)

    def _visit_add(self, node):
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('ADD')

    def _visit_sub(self, node):
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('SUB')

    def _visit_mul(self, node):
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('MUL')

    def _visit_div(self, node):
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('DIV')

    def _visit_mod(self, node):
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('MOD')

    def _visit_eq(self, node):
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('CMP_EQ')

    def _visit_lt(self, node):
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('CMP_LT')

    def _visit_gt(self, node):
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('CMP_GT')

    def _visit_le(self, node):
        # a <= b  <=>  not (a > b)
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('CMP_GT')
        self.emit('NOT')

    def _visit_ge(self, node):
        # a >= b  <=>  not (a < b)
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('CMP_LT')
        self.emit('NOT')

    def _visit_neq(self, node):
        # a != b <=> not (a == b)
        self._visit(node.children[0])
        self._visit(node.children[1])
        self.emit('CMP_EQ')
        self.emit('NOT')

    def _visit_call_expr(self, node):
        func_node = node.children[0]
        args_node = node.children[1] if len(node.children) > 1 else None

        func_name = None
        if hasattr(func_node, 'data') and func_node.data == 'var':
            func_name = func_node.children[0].value
        elif isinstance(func_node, str):
            func_name = func_node

        if func_name == 'print':
            if args_node and hasattr(args_node, 'children'):
                for arg in args_node.children:
                    self._visit(arg)
                    self.emit('PRINT')
            return

        # Handle user function call
        arg_count = 0
        if args_node and hasattr(args_node, 'children'):
            arg_count = len(args_node.children)
            for arg in args_node.children:
                self._visit(arg)

        if func_name:
             self.emit('CALL', f"func_{func_name}", arg_count)
        else:
             # Try to handle indirect call or method?
             # For now, simplistic.
             pass

    def _visit_function_def(self, node):
        name = node.children[0].value
        # Skip over function body
        end_label = f"end_func_{name}"
        self.emit('JUMP', end_label)

        func_label = f"func_{name}"
        self.define_label(func_label)
        self.function_labels[name] = func_label

        body_idx = 1
        if len(node.children) > 1 and node.children[1] and hasattr(node.children[1], 'data') and node.children[1].data == 'param_list':
             params = [t.value for t in node.children[1].children]
             for param in reversed(params):
                 idx = self.get_var_index(param)
                 self.emit('STORE_VAR', idx)
             body_idx = 2

        self._visit(node.children[body_idx])
        self.emit('RETURN')
        self.define_label(end_label)

    def _visit_return_stmt(self, node):
        if node.children:
            self._visit(node.children[0])
        else:
            idx = self.get_const_index(None, "Unit")
            self.emit('PUSH_CONST', idx)
        self.emit('RETURN')

    def _visit_if_stmt(self, node):
        # children: condition, then_block, [else_block...]
        cond = node.children[0]
        self._visit(cond)

        else_label = f"else_{len(self.code)}"
        end_label = f"end_if_{len(self.code)}"

        self.emit('JUMP_IF_FALSE', else_label)

        self._visit(node.children[1]) # Then block
        self.emit('JUMP', end_label)

        self.define_label(else_label)
        if len(node.children) > 2:
            self._visit(node.children[2]) # Else block

        self.define_label(end_label)

    def _visit_while_stmt(self, node):
        start_label = f"while_start_{len(self.code)}"
        end_label = f"while_end_{len(self.code)}"

        self.define_label(start_label)
        self._visit(node.children[0]) # Condition
        self.emit('JUMP_IF_FALSE', end_label)

        self._visit(node.children[1]) # Body
        self.emit('JUMP', start_label)

        self.define_label(end_label)


class BytecodeDisassembler:
    def disassemble(self, data):
        output = []
        if data[:4] != b'ARKB':
            raise ValueError("Invalid magic bytes")

        version = data[4]
        # skip 3 reserved

        offset = 8

        # Constant Pool
        cp_count = struct.unpack_from('>H', data, offset)[0]
        offset += 2

        output.append("Constant Pool:")
        for i in range(cp_count):
            tag = data[offset]
            offset += 1
            if tag == 0x01: # Int
                val = struct.unpack_from('>q', data, offset)[0]
                offset += 8
                output.append(f"  {i}: Int({val})")
            elif tag == 0x02: # String
                slen = struct.unpack_from('>H', data, offset)[0]
                offset += 2
                sval = data[offset:offset+slen].decode('utf-8')
                offset += slen
                output.append(f"  {i}: String('{sval}')")
            elif tag == 0x03: # Bool
                bval = data[offset] != 0
                offset += 1
                output.append(f"  {i}: Bool({bval})")
            elif tag == 0x04: # Float
                fval = struct.unpack_from('>d', data, offset)[0]
                offset += 8
                output.append(f"  {i}: Float({fval})")

        output.append("\nInstructions:")
        pc = 0
        code_data = data[offset:]
        while pc < len(code_data):
            opcode = code_data[pc]
            name = OPCODE_NAMES.get(opcode, f"UNKNOWN({opcode})")

            instr_str = f"{pc:04x}: {name}"
            pc += 1

            # Operands
            if name in ['PUSH_CONST', 'LOAD_VAR', 'STORE_VAR', 'JUMP', 'JUMP_IF_FALSE']:
                op = struct.unpack_from('>H', code_data, pc)[0]
                instr_str += f" {op}"
                pc += 2
            elif name == 'CALL':
                addr = struct.unpack_from('>H', code_data, pc)[0]
                pc += 2
                argc = code_data[pc]
                pc += 1
                instr_str += f" addr={addr} argc={argc}"

            output.append(instr_str)

        return "\n".join(output)
