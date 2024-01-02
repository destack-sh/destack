from uuid import UUID

from asgiref.sync import sync_to_async

from bench import models
from bench.language import Module, libs
from bench.language.builtin import symbolx_lib
from bench.language.const import INTERP_NODE_TYPES, ModuleReference
from bench.models import Bench, BenchVersion, packer
from bench.proto import wire, wiring
from bench.utils.func import to_uuid

_cached_modules: dict[ModuleReference | UUID, tuple[wire.ModuleTreeData, models.Bench]] = {}


async def read_module(ref: ModuleReference | UUID) -> tuple[wire.ModuleTreeData, models.Bench]:
    if ref in _cached_modules:
        return _cached_modules[ref]
    id = ref if isinstance(ref, UUID) else ref.id
    if id:
        bench_version = await BenchVersion.objects.aget(id=id)
    else:
        owner, bench = ref.name.split(".")
        if ref.version != "x":
            raise RuntimeError("versioned module fetch not supported (must be head)")
        bench_version = (
            await Bench.objects.filter(slug=bench)
            .filter(
                models.Q(organization__owner_slug_id=owner) | models.Q(user__owner_slug_id=owner)
            )
            .select_related("head", "user", "organization")
            .aget()
        )
        bench_version = bench_version.head
    module = await sync_to_async(packer.pack_module_host)(bench_version, excluded=INTERP_NODE_TYPES)
    if bench_version.committed:
        _cached_modules[ref] = module, bench_version.bench
    return module, bench_version.bench


async def interp_module(ref: ModuleReference | UUID) -> tuple[Module, models.Bench]:
    module, bench = await read_module(ref)
    module = wiring.unpack_node_inline(
        module.nodes, my_root=to_uuid(module.module.ck), parent=None, exclude=INTERP_NODE_TYPES
    )
    for dependency in libs.DEFAULT_MODULES.values():
        module.add_dependency(dependency)
    module.add_builtin(symbolx_lib.files.get("builtins"))
    module._interp_rec()
    return module, bench
