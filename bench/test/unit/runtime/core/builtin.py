from bench.language import sync_node
from bench.language.builtin import make_builtins
from bench.test.unit.conftest import RuntimeHandle


async def test_sync_builtins(hosted_runtime: RuntimeHandle) -> None:
    """Sync the Builtins page."""
    Builtins = make_builtins(hosted_runtime.session)
    sync_node(
        parent=hosted_runtime.package,
        old_root=hosted_runtime.package.blocks.get("Builtins"),
        new_root=Builtins,
    )
    assert hosted_runtime.session.tx.has_edits
    await hosted_runtime.commit()

    sync_node(
        parent=hosted_runtime.package,
        old_root=hosted_runtime.package.blocks.get("Builtins"),
        new_root=Builtins,
    )
    assert not hosted_runtime.session.tx.has_edits
    await hosted_runtime.commit()
