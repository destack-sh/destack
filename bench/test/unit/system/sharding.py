import pytest

from bench.system.sharding import host_map_from_string, host_map_to_string


@pytest.mark.parametrize(
    "host_map_str",
    [
        "eu-zurich=localhost:8080,eu-frankfurt=localhost:8081",
        "eu-frankfurt=aws-eu-frankfurt.host.justbench.com,*=host.justbench.com",
    ],
)
def test_roundtrip_hostmap(host_map_str: str):
    host_map = host_map_from_string(host_map_str)
    rendered = host_map_to_string(host_map)
    assert rendered == host_map_str
