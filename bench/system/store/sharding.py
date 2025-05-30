from typing import TYPE_CHECKING, Literal

from attr import dataclass
from fastuuid import UUID

from bench.language import (
    BEGINNING_OF_TIME,
    REGION,
    VERSION,
    Bench,
    BenchStatus,
    Package,
    PackageType,
    Region,
    Session,
)
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


def make_system_database(region: Region, pg_url: str) -> Database:
    system_session = Session()
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        status=BenchStatus.RUNNING,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    system_package_stub = Package(
        parent=system_bench_stub,
        type=PackageType.HOME,
        id=UUID(int=1),
        name="Home",
        slug="home",
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    database = Database(
        parent=system_package_stub,
        name="Database",
        version=VERSION,
        sql_url=pg_url,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    return database


def get_global_database_from_env() -> Database:
    """Get the default global database configured in the environment"""
    pg = get_from_env("GLOBAL_PG_URL", description="Global Postgres connection string")
    pg_url = pg.split("|", maxsplit=1)[0]
    return make_system_database(REGION, pg_url)


def get_main_database_from_env(region: Region = REGION) -> Database:
    """Get the default regional database configured in the environment"""
    from .sharding import DATABASE_MAP

    return DATABASE_MAP.get(region)
