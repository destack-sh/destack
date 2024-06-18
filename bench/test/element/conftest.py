# ruff: noqa: E402

import pytest

from bench.test.conftest import setup_test

# NOTE: must run setup_test() before importing from bench
setup_test()

from bench.language import Bench, Session
from bench.language.graph import NodeSuperGraph


@pytest.fixture()
async def session(request):
    supergraph = NodeSuperGraph(root_ptr=None)
    bench = Bench(slug=f"test-{request.node.name}", name=request.node.name, _supergraph=supergraph)
    supergraph._root_ptr = bench.to_ref()
    session = Session(_supergraph=bench._supergraph)
    async with session:
        yield session
