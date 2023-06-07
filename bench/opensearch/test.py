from uuid import UUID

import opensearchpy.exceptions
import pytest

from bench.opensearch.client import os_client
from bench.opensearch.index import create_index
from bench.opensearch.type import IndexType

MOCK_PROJECT_ID = UUID("00000000-0000-0000-0000-000000000000")


@pytest.mark.parametrize("index_t", IndexType)
def test_create_indices(index_t: IndexType):
    project_id = MOCK_PROJECT_ID if index_t.is_project_scoped else None
    try:
        os_client.indices.delete(index=index_t.get_index_name(project_id))
    except opensearchpy.exceptions.NotFoundError:
        pass
    create_index(index_t, project_id)
