from uuid import UUID

import pytest

from bench.opensearch.client import os_client
from bench.opensearch.index import create_global_index, create_bench_index
from bench.opensearch.type import IndexType

MOCK_PROJECT_ID = UUID("00000000-0000-0000-0000-000000000000")


@pytest.mark.parametrize("index_t", IndexType)
def test_create_indices(index_t: IndexType):
    os_client.indices.delete("test-bench-global", ignore=[404])
    os_client.indices.delete("test-bench-user", ignore=[404])
    create_global_index(name="test-bench-global")
    create_bench_index(MOCK_PROJECT_ID, name="test-bench-user")
