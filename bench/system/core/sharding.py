import abc
from typing import TYPE_CHECKING, Literal, Mapping
from uuid import UUID

from attr import dataclass

from bench.language import Region
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.language import Database

#
# Databases (by Region)
#


@dataclass(slots=True)
class DatabaseInfo:
    pg_url: str

    def render(self) -> str:
        return self.pg_url

    @staticmethod
    def parse(region_url: str) -> "DatabaseInfo":
        """Parses a region URL like 'postgresql://user:pass@host/db'."""
        return DatabaseInfo(pg_url=region_url)


class DatabaseMap:
    """
    Maps Regions to regional DBs.
    """

    def __init__(self, database_map: dict[Region | Literal["*"], "DatabaseInfo | Database"]):
        self._database_info_by_region: dict[Region | Literal["*"], DatabaseInfo] = {}
        self._database_by_region: dict[Region | Literal["*"], Database] = {}
        for region, database_or_info in database_map.items():
            if isinstance(database_or_info, DatabaseInfo):
                self._database_info_by_region[region] = database_or_info
            else:
                self._database_by_region[region] = database_or_info

    def __str__(self) -> str:
        return database_map_to_string(self)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {database_map_to_string(self) or '<empty>'}>"

    def get_info(self, region: Region) -> DatabaseInfo:
        """Gets the region info for the given region (error if none)."""
        database_info = self._database_info_by_region.get(region)
        if database_info is None:
            database_info = self._database_info_by_region.get("*")
        if database_info is None:
            raise LookupError(f"no database info for {region.bench_name} in {self!r}")
        return database_info

    def get(self, region: Region) -> "Database":
        """Gets the Database for the given region (error if none)."""
        database = self._database_by_region.get(region)
        if database is None:
            database = self._database_by_region.get("*")
        if database is None:
            from bench.system.core import make_system_database

            database_info = self.get_info(region)
            database = make_system_database(region, database_info.pg_url)
            self._database_by_region[region] = database
        return database


def get_database_map_from_string(map_str: str) -> "DatabaseMap":
    """
    Parses a map string like:
        '*=postgresql://user:pass@host/db'
        'eu-zurich=postgresql://user:pass@host/db;eu-frankfurt=postgresql://user:pass@host/db'
    """
    database_map = {}
    for mapping_str in map_str.split(";"):
        database_str, database_info_str = mapping_str.split("=", 1)
        region = Region.get_by_slug(database_str) if database_str != "*" else database_str
        database_info = DatabaseInfo.parse(database_info_str.strip())
        database_map[region] = database_info
    return DatabaseMap(database_map=database_map)


def database_map_to_string(database_map: "DatabaseMap") -> str:
    """Renders a region map back into a string."""
    return ";".join(
        f"{k.slug if isinstance(k, Region) else k}={v.render()}"
        for k, v in database_map._database_info_by_region.items()
    )


def get_database_map_from_env() -> "DatabaseMap":
    """Parses the REGIONAL_PG_MAP from the environment."""
    region_map_str = get_from_env("REGIONAL_PG_MAP", description="Region map for postgres sharding")
    return get_database_map_from_string(region_map_str)


DATABASE_MAP = get_database_map_from_env()

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
    def parse(host_url: str) -> "HostInfo":
        """Parses a host URL like 'host.justbench.com:8080/443'."""
        if host_url.endswith("s"):
            ssl = True
            host_url = host_url[:-1]
        else:
            ssl = False
        domain, ports_str = host_url.split(":", maxsplit=1)
        grpc_port, grpc_web_port = ports_str.split("/", maxsplit=1)
        return HostInfo(
            host_domain=domain, grpc_port=int(grpc_port), grpc_web_port=int(grpc_web_port), ssl=ssl
        )


class HostMap(abc.ABC):
    """
    Maps Regions and Bench IDs to Host URLs.
    """

    @abc.abstractmethod
    def get(self, bench_id: UUID, region: Region) -> HostInfo:
        """Gets the host info for the given bench ID and region (error if none)."""
        ...


class StaticHostMap(HostMap):
    """A static host map backed by a dictionary."""

    def __init__(
        self,
        hosts: Mapping[UUID | Region | Literal["*"], HostInfo],
    ):
        self._hosts = hosts

    def __str__(self) -> str:
        return host_map_to_string(self)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {host_map_to_string(self) or '<empty>'}>"

    def get(self, bench_id: UUID, region: Region) -> HostInfo:
        """Gets the host info for the given bench ID and region (error if none)."""
        host_info = self._hosts.get(bench_id)
        if host_info is None:
            host_info = self._hosts.get(region)
        if host_info is None:
            host_info = self._hosts.get("*")
        if host_info is None:
            raise LookupError(f"no host info for {region.bench_name} in {self!r}")
        return host_info


def get_host_map_from_string(host_map_str: str) -> StaticHostMap:
    """
    Parses a host map string like:
        'eu-zurich=localhost:60061/8080,eu-frankfurt=localhost:60061/8081'
        'eu-frankfurt=aws-eu-frankfurt.host.justbench.com:8080/443,*=host.justbench.com:8080/443'
        '01234567-8910-0000-0000-000000000000=host1.justbench.com:8080/443,eu-frankfurt=host2.justbench.com:8080/443'
    """
    hosts: dict[UUID | Region | Literal["*"], HostInfo] = {}

    for mapping_str in host_map_str.split(","):
        key_str, host_info_str = mapping_str.split("=", 1)
        host_info = HostInfo.parse(host_info_str.strip())
        try:
            bench_id = UUID(key_str)
            hosts[bench_id] = host_info
        except ValueError:
            region = Region.get_by_slug(key_str) if key_str != "*" else key_str
            hosts[region] = host_info

    return StaticHostMap(hosts=hosts)


def host_map_to_string(host_map: StaticHostMap) -> str:
    """Renders a host map back into a string."""
    return ",".join(
        f"{k.slug if isinstance(k, Region) else k}={v.render()}" for k, v in host_map._hosts.items()
    )


def get_host_map_from_env() -> StaticHostMap:
    """Parses the HOST_MAP from the environment."""
    host_map_str = get_from_env("HOST_MAP", description="Host map for sharding")
    return get_host_map_from_string(host_map_str)


HOST_MAP = get_host_map_from_env()
