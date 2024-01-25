import uuid

from bench.language.node import Bench, Package
from bench.language.const import UUID_NAMESPACE, VERSION

symbolx_bench = Bench(
    name="SymbolX", slug="symbolx", id=uuid.uuid5(UUID_NAMESPACE, "builtin:symbolx.bench")
)
symbolx_package = Package(parent=symbolx_bench, id=uuid.uuid5(symbolx_bench.id, VERSION))
DEFAULT_DEPENDENCIES = {symbolx_bench.slug: symbolx_package.id}
