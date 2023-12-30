from uuid import UUID

from asgiref.sync import sync_to_async

from bench import models
from bench.language import Module, libs
from bench.language.builtin import symbolx_lib
from bench.language.const import ModuleReference, INTERP_NODE_TYPES
from bench.models import ProjectVersion, Project, packer
from bench.proto import wire, wiring

_cached_modules: dict[ModuleReference | UUID, tuple[wire.ModuleTreeData, models.Project]] = {}


async def read_module(ref: ModuleReference | UUID) -> tuple[wire.ModuleTreeData, models.Project]:
    if ref in _cached_modules:
        return _cached_modules[ref]
    id = ref if isinstance(ref, UUID) else ref.id
    if id:
        project_version = await ProjectVersion.objects.aget(id=id)
    else:
        owner, project = ref.name.split(".")
        if ref.version != "x":
            raise RuntimeError("versioned module fetch not supported (must be head)")
        project_version = (
            await Project.objects.filter(slug=project)
            .filter(
                models.Q(organization__owner_slug_id=owner) | models.Q(user__owner_slug_id=owner)
            )
            .select_related("head", "user", "organization")
            .aget()
        )
        project_version = project_version.head
    module = await sync_to_async(packer.pack_module_host)(
        project_version, excluded=INTERP_NODE_TYPES
    )
    if project_version.committed:
        _cached_modules[ref] = module, project_version.project
    return module, project_version.project


async def interp_module(ref: ModuleReference | UUID) -> tuple[Module, models.Project]:
    module, project = await read_module(ref)
    module = wiring.unpack_node_inline(module.nodes, parent=None, exclude=INTERP_NODE_TYPES)
    for dependency in libs.DEFAULT_MODULES.values():
        module.add_dependency(dependency)
    module.add_builtin(symbolx_lib.files.get("builtins"))
    module._interp_rec()
    return module, project
