from opensearchpy import AsyncOpenSearch

from bench.utils.utils import get_from_env, IS_DEBUG

OS_HOST = get_from_env("LOCAL_OS_HOST", alt="GLOBAL_OS_HOST")
OS_NAME = get_from_env("LOCAL_OS_NAME", optional=True)
OS_PORT = get_from_env("LOCAL_OS_PORT", default=9200, type_cast=int, alt="GLOBAL_OS_PORT")
OS_USERNAME = get_from_env("LOCAL_OS_USERNAME", alt="GLOBAL_OS_USERNAME")
OS_PASSWORD = get_from_env("LOCAL_OS_PASSWORD", alt="GLOBAL_OS_PASSWORD")

os_client = AsyncOpenSearch(
    hosts=[{"host": OS_HOST, "port": OS_PORT}],
    http_auth=(OS_USERNAME, OS_PASSWORD),
    http_compress=True,
    verify_certs=not IS_DEBUG,
    ssl_show_warn=False,
    use_ssl=True,
)


def get_os_errors(ret: dict) -> list[dict]:
    return [i for i in ret["items"] if i.get("index", {}).get("error")]
