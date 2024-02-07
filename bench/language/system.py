import uuid

from bench.language.node import Bench, Package
from bench.language.const import UUID_NAMESPACE, VERSION

bench_bench = Bench(name="Bench", slug="bench", id=uuid.uuid5(UUID_NAMESPACE, "builtin:bench"))
bench_package = Package(parent=bench_bench, id=uuid.uuid5(bench_bench.id, VERSION))
