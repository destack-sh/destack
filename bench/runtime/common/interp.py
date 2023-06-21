import asyncio
from dataclasses import dataclass
from functools import partial
from typing import Awaitable, Callable, Optional, cast
from uuid import UUID

import structlog

from bench import bench as language
from bench.bench import Module, wire
from bench.bench.core import ModuleReference, Session
from bench.utils.func import wrap_task

logger = structlog.get_logger(__name__)


@dataclass(repr=False, slots=True)
class InterpModule:
    module: Optional[language.Module]
    issues: list[language.Issue]
    tree: Optional[wire.ModuleTree]

    def __str__(self):
        return str(self.module)

    def __repr__(self):
        return f"<InterpModule {self}>"


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


ModuleFetcher = Callable[[ModuleReference], Awaitable[wire.ModuleTreeData]]


class LanguageInterpreter:
    def __init__(self, fetcher: ModuleFetcher):
        self.fetcher = fetcher
        self.interp_dependencies_cached: dict[ModuleReference, InterpModule] = {}

    async def interp_requirement_rec(self, ref: ModuleReference) -> InterpModule:
        """Fetch and interpret the requirement module (incl. transitive deps)"""
        if ref in self.interp_dependencies_cached:
            return self.interp_dependencies_cached[ref]
        source = await self.fetcher(ref)
        requirements = get_requirements(source)
        dependencies = await asyncio.gather(
            *[self.interp_requirement_rec(req) for req in requirements]
        )
        interp = await asyncio.get_event_loop().run_in_executor(
            None, partial(interp_module, source, [m.module for m in dependencies], None)
        )
        if interp.module.errors:
            # not good, but we can still try to use the module?
            logger.warn("module.requirement.failed", ref=ref, interp=interp)
        self.interp_dependencies_cached[ref] = interp
        return interp

    async def interp_requirements(self, requirements: set[ModuleReference]) -> list[InterpModule]:
        # return immediately if all cached (saves context switching)
        all_cached = all(r.id in self.interp_dependencies_cached for r in requirements)
        if all_cached:
            return [self.interp_dependencies_cached[r] for r in requirements]
        dependencies = await asyncio.gather(
            *[self.interp_requirement_rec(r) for r in requirements], return_exceptions=False
        )
        return cast(list[InterpModule], dependencies)

    async def interp(self, source: wire.ModuleTreeData, session: Optional[Session]) -> InterpModule:
        dependencies = await self.interp_requirements(get_requirements(source))
        return interp_module(source, [m.module for m in dependencies], session)


DEFAULT_REQUIREMENTS = ("symbolx.std", "openai.std", "anthropic.std")


def get_requirements(source: wire.ModuleTreeData) -> set[ModuleReference]:
    """Returns the set of module ids required by the given module source (not transitive)"""
    requirements_ids: set[ModuleReference] = set()
    for node in source.nodes:
        if isinstance(node, wire.RequirementData):
            if not isinstance(node.reference_module.id, UUID):
                raise ValueError(f"requirement must specify reference module id: {node}")
            requirements_ids.add(node.reference_module)

    # TODO @Cleanup: manage requirements properly
    if source.name not in DEFAULT_REQUIREMENTS:
        for req in DEFAULT_REQUIREMENTS:
            requirements_ids.add(ModuleReference(name=req, version="x", id=None))

    return requirements_ids


def interp_module(
    source: wire.ModuleTreeData, dependencies: list[Module], session: Optional[Session]
) -> InterpModule:
    """Interprets the given module source with the given dependencies"""
    logger.debug("module.interp", module=source)
    module = wire.unpack_module(source, session=session)
    for dependency in dependencies:
        module.add_dependency(dependency)
    logger.debug("module.interp.index", module=module)
    module.index()
    module.interp()
    logger.debug("module.interp.done", module=module)
    tree = wire.ModuleTree(wire.pack_module(module).nodes)
    return InterpModule(module=module, tree=tree, issues=module.issues)
