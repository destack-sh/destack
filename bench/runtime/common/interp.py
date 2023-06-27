from dataclasses import dataclass
from dataclasses import dataclass
from typing import Optional

import structlog

from bench import bench as language
from bench.bench import Module, wire
from bench.bench.core import Session

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
