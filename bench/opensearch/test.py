from uuid import UUID

import pytest

from bench.opensearch.client import os_client
from bench.opensearch.index import create_global_index, create_bench_index
from bench.opensearch.type import IndexType

MOCK_PROJECT_ID = UUID("00000000-0000-0000-0000-000000000000")


def _delete_index_if_exists(*indices: str):
    for index in indices:
        os_client.indices.delete(index, ignore=[404])


@pytest.mark.parametrize("index_t", IndexType)
def test_create_indices(index_t: IndexType):
    _delete_index_if_exists("test-bench-global", "test-bench-user")
    create_global_index(name="test-bench-global")
    create_bench_index(MOCK_PROJECT_ID, name="test-bench-user")
