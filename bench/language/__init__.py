from itertools import chain

from ..utils.func import get_subclasses
from .blob import Blob
from .const import (
    ConditionalOp,
    IssueType,
    QueryEngine,
    ScheduleType,
    SessionAccessLevel,
    SortMode,
    SortOp,
    StatementType,
    TriggerType,
    TypeFlag,
    TypeHint,
    TypeTag,
    ViewLayout,
)
from .database import HasDatabase, Record
from .edit import render, render_as_python
from .expression import (
    A,
    Aggregation,
    C,
    Conditional,
    E,
    Expression,
    S,
    Sort,
)
from .field import Field, HasFields, HasType, ResolvedField, Type, TypeStorageFormat
from .file import File
from .issue import BenchError, Issue
from .module import Module, Node, ScopeNode, Struct
from .reference import NodeVisitor
from .run import HasRun, Run, RunError
from .secret import Secret
from .session import LogEntry, PermissionError, Session
from .statement import Statement
from .tagging import HasTags, Tagging
from .text import HasText
from .trigger import HasTriggers, Trigger
from .value import HasValue
from .view import View

# NOTE! that all these imports are auto-imported as prelude in user code.
#  (maybe we should factor that out...)
__all__ = [
    "A",
    "Aggregation",
    "BenchError",
    "Blob",
    "C",
    "Conditional",
    "ConditionalOp",
    "E",
    "Expression",
    "Field",
    "File",
    "HasDatabase",
    "HasFields",
    "HasRun",
    "HasTags",
    "HasText",
    "HasTriggers",
    "HasType",
    "HasValue",
    "Issue",
    "IssueType",
    "LogEntry",
    "Module",
    "Node",
    "NodeVisitor",
    "PermissionError",
    "QueryEngine",
    "Record",
    "render",
    "render_as_python",
    "ResolvedField",
    "Run",
    "RunError",
    "S",
    "ScheduleType",
    "ScopeNode",
    "Secret",
    "Session",
    "SessionAccessLevel",
    "Sort",
    "SortMode",
    "SortOp",
    "Statement",
    "StatementType",
    "Struct",
    "Tagging",
    "Trigger",
    "TriggerType",
    "Type",
    "TypeFlag",
    "TypeHint",
    "TypeStorageFormat",
    "TypeTag",
    "View",
    "ViewLayout",
]

# after all the imports, we can finalize

# set reflected struct/node properties as static fields
#  (can't do this before because dataclass needs the original class's fields)
for cls in chain(get_subclasses(Node), get_subclasses(Struct)):
    for name, prop in cls.__properties__.items():
        if prop.is_reflected:
            setattr(cls, name, prop)
            prop._as_field  # noqa ensure the reflected field works
