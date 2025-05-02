import pytest

from bench.system.core.sharding import get_host_map_from_string, host_map_to_string


@pytest.mark.parametrize(
    "host_map_str",
    [
        "eu-zurich=localhost:60061/8080s,eu-frankfurt=localhost:60061/8081",
        "eu-frankfurt=aws-eu-frankfurt.host.justbench.com:8080/443s,*=host.justbench.com:8080/443",
    ],
)
def test_roundtrip_hostmap(host_map_str: str):
    host_map = get_host_map_from_string(host_map_str)
    rendered = host_map_to_string(host_map)
    assert rendered == host_map_str
