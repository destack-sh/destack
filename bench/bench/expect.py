from __future__ import annotations

from dataclasses import field
from typing import Union, Optional, Self

from bench.bench.const import ExpectationModifier
from bench.bench.core import Scope, Symbol, Statement
from bench.bench.core import node, SymbolBase, StatementReference
from bench.bench.issue import IssueType

Expectable = Union["Expectation", "Task", "Dataset", "Code"]


@node
class IsExpectable:
    """Symbols that can define expectations"""

    modifier: Optional[ExpectationModifier] = None


@node
class HasExpectations(SymbolBase, IsExpectable):
    """Symbols we can attach expectations to"""

    description: Optional[str] = None
    expectations: list[Expectable] = field(default_factory=list)
    resolved_expectations: list[Expectable] | None = None

    def expect(self, expectation: Expectable) -> "Self":
        self.expectations.append(expectation)
        return self

    def like(self, expectation: Expectable) -> "Self":
        # same
        raise NotImplementedError

    def unlike(self, expectation: Expectable) -> "Self":
        # same
        raise NotImplementedError

    def _clear(self) -> None:
        self.resolved_expectations = None

    def _interp(self, scope: Scope) -> None:
        if self.resolved_expectations is not None:
            return
        from bench.bench.type import HasType

        resolved_expectations = [*self.expectations]
        if isinstance(self, HasType):
            # inline union expectations
            for base in self.bases:
                if isinstance(base.reference, HasExpectations):
                    resolved_expectations.extend(base.reference.expectations)
        self.resolved_expectations = resolved_expectations


@node
class Expectation(Symbol, HasExpectations):
    reference: StatementReference | Statement | None = None
    description: Optional[str] = None

    def _interp(self, scope: Scope) -> None:
        # resolve reference
        if self.reference is not None:
            resolved = scope.lookup_symbol(self.reference)
            if resolved is None:
                self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
            else:
                self.reference = resolved
        # interp
        HasExpectations._interp(self, scope)
        if isinstance(self.reference, HasExpectations):
            self.resolved_expectations.extend(self.reference.expectations)

    def _clear(self) -> None:
        Symbol._clear(self)
        HasExpectations._clear(self)
