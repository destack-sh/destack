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
from .expression import A, C, E, Expression, S
from .field import Field, HasFields, HasType, ResolvedField, Type, TypeStorageFormat
from .file import File
from .issue import BenchError, Issue
from .module import Module, Node, Property, ScopeNode, Struct, complete_setup
from .projection import NodeVisitor
from .run import HasRun, Run, RunError
from .secret import Secret
from .session import LogEntry, PermissionError, Session
from .statement import Statement
from .tagging import HasTags, Tagging
from .text import HasText
from .trigger import HasTriggers, Trigger
from .value import HasValue
from .view import View

# NOTE! *All* these imports are auto-imported as prelude in user code.
__all__ = [
    "A",
    "BenchError",
    "Blob",
    "C",
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
    "Property",
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
complete_setup()
