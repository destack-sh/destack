from opensearchpy import AsyncOpenSearch, OpenSearch

from bench.utils.utils import LOCAL, get_from_env

OS_HOST = get_from_env("LOCAL_OS_HOST", default="localhost", alt="GLOBAL_OS_HOST")
OS_NAME = get_from_env("LOCAL_OS_NAME", optional=True)
OS_PORT = get_from_env("LOCAL_OS_PORT", default=9200, type_cast=int, alt="GLOBAL_OS_PORT")
OS_USERNAME = get_from_env("LOCAL_OS_USERNAME", default="admin", alt="GLOBAL_OS_USERNAME")
OS_PASSWORD = get_from_env("LOCAL_OS_PASSWORD", default="admin", alt="GLOBAL_OS_PASSWORD")

os_client_sync = OpenSearch(
    hosts=[{"host": OS_HOST, "port": OS_PORT}],
    http_auth=(OS_USERNAME, OS_PASSWORD),
    http_compress=True,
    verify_certs=not LOCAL,
    ssl_show_warn=False,
    use_ssl=True,
)
os_client = AsyncOpenSearch(
    hosts=[{"host": OS_HOST, "port": OS_PORT}],
    http_auth=(OS_USERNAME, OS_PASSWORD),
    http_compress=True,
    verify_certs=not LOCAL,
    ssl_show_warn=False,
    use_ssl=True,
)
