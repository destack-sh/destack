# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.session.module import (
    Module,
)

class Document:
    """One document accepted by formatter operations."""

    """Repository module at one immutable revision."""
    @staticmethod
    def module(module: Module) -> Document: ...

    """Ad hoc source text."""
    @staticmethod
    def text(path: str, text: str) -> Document: ...

    @property
    def kind(self) -> str: ...

    @property
    def module_module(self) -> Module | None: ...

    @property
    def path(self) -> str | None: ...

    @property
    def text_text(self) -> str | None: ...

class FormatRequest:
    """One formatter request."""

    def __init__(self, document: Document) -> None: ...

    """Document to format."""
    @property
    def document(self) -> Document: ...

class FormatOutput:
    """One formatter output."""

    def __init__(self, text: str) -> None: ...

    """Formatted source text."""
    @property
    def text(self) -> str: ...
