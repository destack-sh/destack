from destack.language import DatabaseInfo, DatabaseType, Region
from destack.utils.env import get_from_env

from .database import StaticDatabaseProvider
from .galaxy import StaticGalaxyProvider


def get_global_database_from_env() -> DatabaseInfo:
    """Get the default global database configured in the environment"""
    sql_url = get_from_env("GLOBAL_DATABASE_URL", description="Global database URL")
    return DatabaseInfo(
        type=DatabaseType.POSTGRES,
        connection_url=sql_url,
        region=Region.ZURICH,
        external_name="destack-global",
    )


def get_database_provider_from_env() -> StaticDatabaseProvider:
    """Get the default database provider configured in the environment"""
    db_map_str = get_from_env("DATABASE_MAP", description="Database map for sharding")
    return StaticDatabaseProvider.parse(db_map_str)


DATABASE_PROVIDER = get_database_provider_from_env()


def get_galaxy_provider_from_env() -> StaticGalaxyProvider:
    """Parses the GALAXY_MAP from the environment."""
    galaxy_map_str = get_from_env("GALAXY_MAP", description="Galaxy map for sharding")
    return StaticGalaxyProvider.parse(galaxy_map_str)


GALAXY_PROVIDER = get_galaxy_provider_from_env()
