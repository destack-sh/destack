import asyncio
from dataclasses import dataclass
from functools import partial
from itertools import chain
from typing import Awaitable, Callable, Optional, cast
from uuid import UUID

import structlog

from bench import language
from bench.language import SymbolType, wire
from bench.language.const import IssueCollector
from bench.language.wire import ModuleReference
from bench.utils.func import wrap_task

logger = structlog.get_logger(__name__)


@dataclass(repr=False, slots=True)
class InterpModule:
    module: Optional[language.Module]
    errors: list[language.Issue]
    dependencies: list[language.Module]
    committed: bool

    def __str__(self):
        return str(self.module)

    def __repr__(self):
        return f"<InterpModule {self}>"


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


ModuleFetcher = Callable[[UUID, int], Awaitable[wire.ModuleData]]


class LanguageInterpreter:
    def __init__(self, fetcher: ModuleFetcher):
        self.fetcher = fetcher
        self.interp_dependencies_cached: dict[UUID, InterpModule] = {}

    async def interp_requirement_rec(self, module_id: UUID) -> InterpModule:
        """Fetch and interpret the requirement module (incl. transitive deps)"""
        if module_id in self.interp_dependencies_cached:
            return self.interp_dependencies_cached[module_id]
        source = await self.fetcher(module_id)
        requirements = get_requirements(source)
        dependencies = await asyncio.gather(
            *[self.interp_requirement_rec(req.id) for req in requirements]
        )
        interp = await asyncio.get_event_loop().run_in_executor(
            None, partial(interp_module, source, [m.module for m in dependencies])
        )
        if interp.errors:
            # not good, but we can still try to use the module?
            logger.warn("module.requirement.failed", interp=interp)
        self.interp_dependencies_cached[module_id] = interp
        return interp

    async def interp_requirements(self, requirements: set[ModuleReference]) -> list[InterpModule]:
        # return immediately if all cached (saves context switching)
        all_cached = all(r.id in self.interp_dependencies_cached for r in requirements)
        if all_cached:
            return [self.interp_dependencies_cached[r.id] for r in requirements]
        dependencies = await asyncio.gather(
            *[self.interp_requirement_rec(r.id) for r in requirements], return_exceptions=False
        )
        return cast(list[InterpModule], dependencies)

    async def interp(self, source: wire.ModuleData) -> InterpModule:
        dependencies = await self.interp_requirements(get_requirements(source))
        return interp_module(source, [m.module for m in dependencies])


def get_requirements(source: wire.ModuleData) -> set[ModuleReference]:
    """Returns the set of module ids required by the given module source (not transitive)"""
    requirements_ids: set[ModuleReference] = set()
    for statement in chain.from_iterable(file.statements for file in source.files):
        if statement.symbol_type == SymbolType.REQUIREMENT:
            if not isinstance(statement.reference_module.id, UUID):
                raise ValueError(f"requirement must specify reference module id: {statement}")
            requirements_ids.add(statement.reference_module)
    return requirements_ids


def interp_module(source: wire.ModuleData, dependencies: list[language.Module]) -> InterpModule:
    """Interprets the given module source with the given dependencies"""
    logger.debug("module.interp", module=source)
    module = wire.wmap_module(source)
    collector = IssueCollector()
    module.index(on_issue=collector)  # nocheckin: check for duplicates in indexing
    module.interp(on_issue=collector)
    errors = [e.to_error() for e in collector.issues]
    logger.debug("module.interp.done", module=module)

    return InterpModule(
        module=module, errors=errors, dependencies=dependencies, committed=source.committed
    )
