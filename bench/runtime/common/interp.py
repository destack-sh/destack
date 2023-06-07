import asyncio
from dataclasses import dataclass
from functools import partial
from itertools import chain
from typing import Awaitable, Callable, Optional, cast
from uuid import UUID

import structlog
from more_itertools import first

from bench import language
from bench.language import wire
from bench.language.interp import ErrorCollector, LookupBy, interp, resolve, sort
from bench.language.parse import REFERENCE_REGEX
from bench.language.type import StatementPath, SymbolType
from bench.language.wire import ModuleReference
from bench.utils.func import wrap_task

logger = structlog.get_logger(__name__)


@dataclass(repr=False, slots=True)
class InterpModule:
    module_idx: Optional[language.ModuleIndex]
    errors: list[language.Error]
    dependencies: list[language.ModuleIndex]
    committed: bool

    def __str__(self):
        return str(self.module_idx)

    def __repr__(self):
        return f"<InterpModule {self}>"

    @property
    def has_user_errors(self):
        """Whether any non-generated errors are present."""
        return any(
            error.statement is None or not error.statement.generated for error in self.errors
        )

    def symbol(self, path: str):
        if path.startswith("."):
            return self.module_idx.symbol(path)
        else:
            match = REFERENCE_REGEX.match(path)
            module_name = match.group("module_owner") + "." + match.group("module_name")
            dependency = next((d for d in self.dependencies if d.module.name == module_name), None)
            if dependency is None:
                raise LookupError(f"could not find dependency {module_name}")
            return dependency.symbol("." + match.group("path") + ":" + match.group("name"))


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
            None, partial(interp_module, source, [m.module_idx for m in dependencies])
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
        return interp_module(source, [m.module_idx for m in dependencies])


def get_requirements(source: wire.ModuleData) -> set[ModuleReference]:
    """Returns the set of module ids required by the given module source (not transitive)"""
    requirements_ids: set[ModuleReference] = set()
    for statement in chain.from_iterable(file.statements for file in source.files):
        if statement.symbol_type == SymbolType.REQUIREMENT:
            if not isinstance(statement.reference_module.id, UUID):
                raise ValueError(f"requirement must specify reference module id: {statement}")
            requirements_ids.add(statement.reference_module)
    return requirements_ids


def lookup_in_dependencies(dependencies: list[language.ModuleIndex]):
    # assumes no conflicting names (checked in resolve)
    dependencies_by_name = {m.module.name: m for m in dependencies}

    def lookup(
        requirement: language.RequirementContent, path: StatementPath | UUID, by: LookupBy
    ) -> language.Scope | None:
        if isinstance(path, UUID):  # lookup in any dependency
            for idx in dependencies:
                scope = idx.scopes.get(path)
                if scope is not None:
                    return scope
            return None
        else:  # lookup in specific requirement
            idx: language.ModuleIndex = dependencies_by_name.get(requirement.module_name)
            if not idx:
                return None
            return idx.get_scope(path, by=by)

    return lookup


def interp_module(
    source: wire.ModuleData, dependencies: list[language.ModuleIndex]
) -> InterpModule:
    """Interprets the given module source with the given dependencies"""
    logger.debug("module.interp", module=source)
    module = wire.wmap_module(source)
    collector = ErrorCollector()
    sort(module)  # for nicer debugging and automatically sorted module index
    idx = resolve(module, lookup_in_module=lookup_in_dependencies(dependencies), on_error=collector)
    interp(idx, on_error=collector)
    errors = [e.to_error() for e in collector.errors]
    logger.debug("module.interp.done", module=idx)

    return InterpModule(
        module_idx=idx, errors=errors, dependencies=dependencies, committed=source.committed
    )


def get_or_create_file(
    idx: language.ModuleIndex, path: str, generated: bool = True
) -> tuple[language.File, bool]:
    file = first((f for f in idx.module.files if f.path == path), None)
    if file is None:
        file = language.File(path=path, statements=[], module=idx.module, generated=generated)
        idx.module.files.append(file)
        return file, True
    else:
        return file, False


def get_file(idx: language.ModuleIndex, path: str) -> Optional[language.File]:
    return first((f for f in idx.module.files if f.path == path), None)
