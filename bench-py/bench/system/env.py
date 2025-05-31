from bench.language import DatabaseInfo, Region, StaticCellRegistry, StaticDatabaseRegistry
from bench.utils.utils import get_from_env


def get_global_database_from_env() -> DatabaseInfo:
    """Get the default global database configured in the environment"""
    sql_url = get_from_env("GLOBAL_DATABASE_URL", description="Global database URL")
    return DatabaseInfo(sql_url=sql_url, region=Region.ZURICH, external_name="bench-global")


def get_database_registry_from_env() -> StaticDatabaseRegistry:
    """Get the default database registry configured in the environment"""
    db_map_str = get_from_env("DATABASE_MAP", description="Database map for sharding")
    return StaticDatabaseRegistry.parse(db_map_str)


DATABASE_REGISTRY = get_database_registry_from_env()


def get_cell_registry_from_env() -> StaticCellRegistry:
    """Parses the CELL_MAP from the environment."""
    cell_map_str = get_from_env("CELL_MAP", description="Cell map for sharding")
    return StaticCellRegistry.parse(cell_map_str)


CELL_REGISTRY = get_cell_registry_from_env()
