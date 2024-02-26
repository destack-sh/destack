from contextlib import asynccontextmanager
from typing import AsyncContextManager

from cachetools import LRUCache, Cache
from opensearchpy import AsyncOpenSearch

from bench.language import Store
from bench.utils.env import IS_DEBUG

_OS_CLIENTS_BY_STORE: Cache[Store, AsyncOpenSearch] = LRUCache(100)


@asynccontextmanager
async def os_client_to_store(store: Store) -> AsyncContextManager[AsyncOpenSearch]:
    os_client = _OS_CLIENTS_BY_STORE.get(store)
    if os_client is None:
        from bench.system.client import USER_OS_HOST

        # TODO :Security :Scalability: route store clients/hosts better :StoreRouting
        os_client = AsyncOpenSearch(
            hosts=[{"host": store.host or USER_OS_HOST, "port": 9200}],
            http_auth=(store.main_credential.username, store.main_credential.password),
            http_compress=True,
            verify_certs=not IS_DEBUG,
            ssl_show_warn=False,
            use_ssl=True,
        )
        _OS_CLIENTS_BY_STORE[store] = os_client
    yield os_client
