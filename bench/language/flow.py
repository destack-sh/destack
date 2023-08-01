from dataclasses import field
from typing import Optional, Union, cast
from uuid import UUID

from bench.language.const import StatementType, TriggerType, TypeTag
from bench.language.core import (
    HasCrud,
    HasSession,
    ModuleNode,
    Scope,
    Statement,
    StatementBase,
    node,
)
from bench.language.type import HasTags, HasType, IssueType, Mapping
from bench.language.utils import Runnable
from bench.utils.utils import required_field


@node
class Trigger(ModuleNode, HasCrud, HasSession):
    """A trigger for a runnable, possibly inside a flow."""

    type: TriggerType = required_field()
    active: bool = True
    mapping: Optional[Mapping] = None
    timezone: Optional[str] = None
    cron: Optional[str] = None
    runnable: Union[Runnable, UUID, None] = None
    scope: Union["HasFlow", UUID, None] = None


@node
class IsFlowable(Runnable, StatementBase):
    """A symbol that can participate in a flow."""

    triggers: list[Trigger] = field(default_factory=list)

    def _clear(self) -> None:
        pass

    def _interp(self, scope: Scope) -> None:
        for trigger in self.triggers:
            # resolve runnable
            if trigger.runnable is not None and not isinstance(trigger.runnable, Runnable):
                resolved = scope.lookup(trigger.runnable)
                if resolved is None:
                    self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
                else:
                    trigger.runnable = resolved
            # resolve scope
            if trigger.scope is not None and not isinstance(trigger.scope, HasFlow):
                resolved = scope.lookup(trigger.scope)
                if resolved is None:
                    self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
                else:
                    trigger.scope = resolved

    def add_trigger(self, trigger: Trigger) -> None:
        raise NotImplementedError

    def remove_trigger(self, trigger: Trigger | UUID) -> None:
        raise NotImplementedError


@node
class HasFlow(Runnable, StatementBase):
    """A symbol that has a flow."""

    def _clear(self) -> None:
        pass

    def _interp(self, scope: Scope) -> None:
        pass


@node
class Flow(HasType, HasFlow, HasTags, Statement):
    """An orchestrated flow of triggered runs."""

    tag: TypeTag = TypeTag.FUNCTION
    type: StatementType = StatementType.CODE
    description: Optional[str] = None
    _is_async: bool = False

    def _clear(self) -> None:
        HasType._clear(self)
        HasFlow._clear(self)
        HasTags._clear(self)
        Statement._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        HasFlow._interp(self, scope)
        HasTags._interp(self, scope)
        Statement._interp(self, scope)

    def to_sync(self) -> "Flow":
        if not self._is_async:
            return self
        return FlowProxy.to_sync(self)

    def to_async(self) -> "Flow":
        if self._is_async:
            return self
        return FlowProxy.to_async(self)


class FlowRunner:
    def __init__(self, flow: Flow):
        self.flow = flow

    def __call__(self, *args, **kwargs):
        raise NotImplementedError


class FlowProxy:  # :SyncProxy
    """
    A simple proxy for Flow to enable to_sync/to_async while keeping the original Flow object.
    """

    def __init__(self, flow: Flow, is_async: bool):
        self._flow = flow
        self._is_async = is_async

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self._flow.__call_async__(*args, **kwargs)
        else:
            return self._flow.__call_sync__(*args, **kwargs)

    def __getattr__(self, item):
        return getattr(self._flow, item)

    @classmethod
    def to_sync(cls, flow: Flow) -> Flow:
        proxy = cls(flow, is_async=False)
        proxy.__call_sync__ = flow.session.async_to_sync(flow.__call_async__)
        return cast(Flow, proxy)

    @classmethod
    def to_async(cls, flow: Flow) -> Flow:
        proxy = cls(flow, is_async=True)
        proxy.__call_async__ = flow.session.sync_to_async(flow.__call_sync__)
        return cast(Flow, proxy)
