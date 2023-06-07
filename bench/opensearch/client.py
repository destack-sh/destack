from opensearchpy import OpenSearch

from bench.settings import (
    LOCAL,
    OPENSEARCH_PASSWORD,
    OPENSEARCH_PORT,
    OPENSEARCH_URL,
    OPENSEARCH_USERNAME,
)

_is_https = OPENSEARCH_URL.startswith("https")
os_client = OpenSearch(
    hosts=[{"host": OPENSEARCH_URL, "port": OPENSEARCH_PORT}],
    http_auth=(OPENSEARCH_USERNAME, OPENSEARCH_PASSWORD),
    http_compress=True,
    verify_certs=not LOCAL,
    ssl_show_warn=False,
    use_ssl=_is_https,
)
