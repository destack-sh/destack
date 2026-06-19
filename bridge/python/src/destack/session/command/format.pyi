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

class FormatRequest:
    """One formatter request."""

    def __init__(self, document: Document) -> None: ...

class FormatOutput:
    """One formatter output."""

    """Formatted source text."""
    @property
    def text(self) -> str: ...
