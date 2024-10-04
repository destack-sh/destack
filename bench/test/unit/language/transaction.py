from bench.test.unit.conftest import RuntimeHandle


async def test_edit_nested_node_data(local_runtime: RuntimeHandle):
    session = local_runtime.session
	# nocheckin