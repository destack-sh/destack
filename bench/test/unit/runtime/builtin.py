from bench.language.builtin import Builtins
from bench.language.node import sync_node
from bench.test.unit.conftest import RuntimeHandle


async def test_sync_builtins(local_runtime: RuntimeHandle) -> None:
    """Sync the Builtins page."""
    sync_node(
        parent=local_runtime.package,
        old_root=local_runtime.package.blocks.get("Builtins"),
        new_root=Builtins,
    )
    assert local_runtime.session.tx.has_edits
    await local_runtime.commit()

    sync_node(
        parent=local_runtime.package,
        old_root=local_runtime.package.blocks.get("Builtins"),
        new_root=Builtins,
    )
    assert not local_runtime.session.tx.has_edits
    await local_runtime.commit()
