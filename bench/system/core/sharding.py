from typing import TYPE_CHECKING, Literal

from attr import dataclass

from bench.language import Region
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.language import Store

#
# Stores (by Region)
#


@dataclass
class StoreInfo:
    pg_url: str
    pg_crypto_key: str

    def render(self) -> str:
        return f"{self.pg_url}|{self.pg_crypto_key}"

    @staticmethod
    def parse(region_uri: str) -> "StoreInfo":
        """Parses a region URI like 'postgresql://user:pass@host/db|crypto_key'."""
        pg_url, crypto_key = region_uri.split("|", maxsplit=1)
        return StoreInfo(pg_url=pg_url, pg_crypto_key=crypto_key)


class StoreMap:
    """
    Maps Regions to Stores.
    """

    def __init__(self, store_map: dict[Region | Literal["*"], "StoreInfo | Store"]):
        self._store_info_by_region: dict[Region | Literal["*"], StoreInfo] = {}
        self._store_by_region: dict[Region | Literal["*"], Store] = {}
        for region, store_or_info in store_map.items():
            if isinstance(store_or_info, StoreInfo):
                self._store_info_by_region[region] = store_or_info
            else:
                self._store_by_region[region] = store_or_info

    def __str__(self) -> str:
        return store_map_to_string(self)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {store_map_to_string(self) or '<empty>'}>"

    def get_info(self, region: Region) -> StoreInfo:
        """Gets the region info for the given region (error if none)."""
        store_info = self._store_info_by_region.get(region)
        if store_info is None:
            store_info = self._store_info_by_region.get("*")
        if store_info is None:
            raise LookupError(f"no store info for {region.bench_name} in {self!r}")
        return store_info

    def get(self, region: Region) -> "Store":
        """Gets the store for the given region (error if none)."""
        store = self._store_by_region.get(region)
        if store is None:
            store = self._store_by_region.get("*")
        if store is None:
            from bench.system.core import make_system_store

            store_info = self.get_info(region)
            store = make_system_store(region, store_info.pg_url, store_info.pg_crypto_key)
            self._store_by_region[region] = store
        return store


def store_map_from_string(region_map_str: str) -> "StoreMap":
    """
    Parses a region map string like:
        '*=postgresql://user:pass@host/db|crypto_key'
        'ZURICH=postgresql://user:pass@host/db|key;FRANKFURT=postgresql://user:pass@host/db|key'
    """
    store_map = {}
    for mapping_str in region_map_str.split(";"):
        store_str, store_info_str = mapping_str.split("=", 1)
        region = Region.get_by_slug(store_str) if store_str != "*" else store_str
        store_info = StoreInfo.parse(store_info_str.strip())
        store_map[region] = store_info
    return StoreMap(store_map=store_map)


def store_map_to_string(region_map: "StoreMap") -> str:
    """Renders a region map back into a string."""
    return ";".join(
        f"{k.slug if isinstance(k, Region) else k}={v.render()}"
        for k, v in region_map._store_info_by_region.items()
    )


def store_map_from_env() -> "StoreMap":
    """Parses the REGIONAL_PG_MAP from the environment."""
    region_map_str = get_from_env("REGIONAL_PG_MAP", description="Region map for postgres sharding")
    return store_map_from_string(region_map_str)


STORE_MAP = store_map_from_env()

#
# Hosts (by Region/Shard)
#


@dataclass
class HostInfo:
    host_domain: str
    grpc_port: int
    grpc_web_port: int
    ssl: bool

    def render(self) -> str:
        return f"{self.host_domain}:{self.grpc_port}/{self.grpc_web_port}{'s' if self.ssl else ''}"

    @staticmethod
    def parse(host_uri: str) -> "HostInfo":
        """Parses a host URI like 'host.justbench.com:8080/443'."""
        if host_uri.endswith("s"):
            ssl = True
            host_uri = host_uri[:-1]
        else:
            ssl = False
        domain, ports_str = host_uri.split(":", maxsplit=1)
        grpc_port, grpc_web_port = ports_str.split("/", maxsplit=1)
        return HostInfo(
            host_domain=domain, grpc_port=int(grpc_port), grpc_web_port=int(grpc_web_port), ssl=ssl
        )


class HostMap:
    """
    Maps Regions to Host URIs.
    """

    def __init__(self, host_by_region: dict[Region | Literal["*"], HostInfo]):
        self._host_by_region = host_by_region

    def __str__(self) -> str:
        return host_map_to_string(self)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {host_map_to_string(self) or '<empty>'}>"

    def get(self, region: Region) -> HostInfo:
        """Gets the host info for the given region (error if none)."""
        host_info = self._host_by_region.get(region)
        if host_info is None:
            host_info = self._host_by_region.get("*")
        if host_info is None:
            raise LookupError(f"no host info for {region.bench_name} in {self!r}")
        return host_info

    __getitem__ = get


def host_map_from_string(host_map_str: str) -> "HostMap":
    """
    Parses a host map string like:
        'eu-zurich=localhost:60061/8080,eu-frankfurt=localhost:60061/8081'
        'eu-frankfurt=aws-eu-frankfurt.host.justbench.com:8080/443,*=host.justbench.com:8080/443'
    """
    host_map = {}
    for mapping_str in host_map_str.split(","):
        region_str, host_info_str = mapping_str.split("=", 1)
        region = Region.get_by_slug(region_str) if region_str != "*" else region_str
        host_info = HostInfo.parse(host_info_str.strip())
        host_map[region] = host_info
    return HostMap(host_by_region=host_map)


def host_map_to_string(host_map: "HostMap") -> str:
    """Renders a host map back into a string."""
    return ",".join(
        f"{k.slug if isinstance(k, Region) else k}={v.render()}"
        for k, v in host_map._host_by_region.items()
    )


def host_map_from_env() -> "HostMap":
    """Parses the HOST_MAP from the environment."""
    host_map_str = get_from_env("HOST_MAP", description="Host map for sharding")
    return host_map_from_string(host_map_str)


HOST_MAP = host_map_from_env()
