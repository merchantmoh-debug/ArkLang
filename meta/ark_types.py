"""
Ark Type System — Core data types for the Ark runtime.

Extracted from ark.py (Phase 72: Structural Hardening).
"""
import sys
from dataclasses import dataclass
from typing import List, Dict, Any, Optional

# slots=True requires Python 3.10+. Gracefully degrade on older versions.
if sys.version_info >= (3, 10):
    def _dataclass_compat(cls):
        return dataclass(slots=True)(cls)
else:
    def _dataclass_compat(cls):
        return dataclass(cls)


class RopeString:
    __slots__ = ('left', 'right', 'val', 'length', '_str_cache')
    def __init__(self, val=None, left=None, right=None):
        self._str_cache = None
        if left is not None and right is not None:
            self.left = left
            self.right = right
            self.val = None
            self.length = len(left) + len(right)
        else:
            self.left = None
            self.right = None
            self.val = val if val is not None else ""
            self.length = len(self.val)

    def __str__(self):
        if self._str_cache is not None:
            return self._str_cache

        parts = []
        stack = [self]
        while stack:
            node = stack.pop()
            if node.val is not None:
                parts.append(node.val)
            else:
                if node.right: stack.append(node.right)
                if node.left: stack.append(node.left)

        self._str_cache = "".join(parts)
        return self._str_cache

    def __repr__(self):
        return f"RopeString(len={self.length})"

    def __len__(self):
        return self.length

    def __add__(self, other):
        if not isinstance(other, (str, RopeString)):
            other = str(other)
        if not isinstance(other, RopeString):
            other = RopeString(other)
        if self.length == 0: return other
        if other.length == 0: return self
        return RopeString(left=self, right=other)

    def __radd__(self, other):
        if not isinstance(other, (str, RopeString)):
            other = str(other)
        if not isinstance(other, RopeString):
            other = RopeString(other)
        if self.length == 0: return other
        if other.length == 0: return self
        return RopeString(left=other, right=self)

    def __getitem__(self, idx):
        return str(self)[idx]

    def __eq__(self, other):
        return str(self) == str(other)

    def __hash__(self):
        return hash(str(self))

    def strip(self):
        return str(self).strip()

    def encode(self, *args, **kwargs):
        return str(self).encode(*args, **kwargs)

    def __lt__(self, other):
        return str(self) < str(other)

    def __gt__(self, other):
        return str(self) > str(other)

    def __le__(self, other):
        return str(self) <= str(other)

    def __ge__(self, other):
        return str(self) >= str(other)


@_dataclass_compat
class ArkValue:
    val: Any
    type: str


UNIT_VALUE = ArkValue(None, "Unit")


class ReturnException(Exception):
    def __init__(self, value):
        self.value = value


@_dataclass_compat
class ArkFunction:
    name: str
    params: List[str]
    body: Any  # Tree node
    closure: 'Scope'


@_dataclass_compat
class ArkClass:
    name: str
    methods: Dict[str, ArkFunction]


@_dataclass_compat
class ArkInstance:
    klass: ArkClass
    fields: Dict[str, ArkValue]


class Scope:
    __slots__ = ('vars', 'parent')

    def __init__(self, parent=None):
        self.vars = {}
        self.parent = parent

    def get(self, name: str) -> Optional[ArkValue]:
        if name in self.vars:
            val = self.vars[name]
            if val.type == "Moved":
                from ark_security import LinearityViolation
                raise LinearityViolation(f"Use of moved variable '{name}'")
            return val
        if self.parent:
            return self.parent.get(name)
        return None

    def set(self, name: str, val: ArkValue):
        self.vars[name] = val

    def mark_moved(self, name: str):
        if name in self.vars:
            self.vars[name] = ArkValue(None, "Moved")
            return
        if self.parent:
            self.parent.mark_moved(name)
