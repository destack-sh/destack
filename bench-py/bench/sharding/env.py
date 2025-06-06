from bench.language import DatabaseInfo, DatabaseType, Region
from bench.utils.utils import get_from_env

from .cell import StaticCellProvider
from .database import StaticDatabaseProvider


def get_global_database_from_env() -> DatabaseInfo:
    """Get the default global database configured in the environment"""
    sql_url = get_from_env("GLOBAL_DATABASE_URL", description="Global database URL")
    return DatabaseInfo(
        type=DatabaseType.POSTGRES,
        sql_url=sql_url,
        region=Region.ZURICH,
        external_name="bench-global",
    )


def get_database_provider_from_env() -> StaticDatabaseProvider:
    """Get the default database provider configured in the environment"""
    db_map_str = get_from_env("DATABASE_MAP", description="Database map for sharding")
    return StaticDatabaseProvider.parse(db_map_str)


DATABASE_PROVIDER = get_database_provider_from_env()


def get_cell_provider_from_env() -> StaticCellProvider:
    """Parses the CELL_MAP from the environment."""
    cell_map_str = get_from_env("CELL_MAP", description="Cell map for sharding")
    return StaticCellProvider.parse(cell_map_str)


CELL_PROVIDER = get_cell_provider_from_env()
