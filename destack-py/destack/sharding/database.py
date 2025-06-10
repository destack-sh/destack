import abc
from collections.abc import Sequence
from typing import TYPE_CHECKING, override

from destack.language import REGION_BY_SLUG, DatabaseInfo, DatabaseType, Region, Space, Tenancy

if TYPE_CHECKING:
    pass


class DatabaseProvider(abc.ABC):
    """A provider of known Destack-level Databases (excluding the global database)."""

    @abc.abstractmethod
    async def acquire(self, region: Region, space: Space) -> "DatabaseInfo":
        """Acquires a new unique (shared) Database for the Space."""
        ...

    @abc.abstractmethod
    async def resolve(
        self, region: Region, cell_name: str, external_name: str
    ) -> "DatabaseInfo | None":
        """Gets the Database for the given region (error if none)."""
        ...

    async def resolve_or_error(
        self, region: Region, cell_name: str, external_name: str
    ) -> "DatabaseInfo":
        """Gets the Database for the given region (error if none)."""
        database_info = await self.resolve(region, cell_name, external_name)
        if database_info is None:
            raise LookupError(
                f'no Database for "{region.slug}/{cell_name}/{external_name}" in {self!r}'
            )
        return database_info


class StaticDatabaseProvider(DatabaseProvider):
    """A static DatabaseProvider with a fixed list of main Databases."""

    def __init__(self, databases: Sequence["DatabaseInfo"]):
        self.databases: tuple[DatabaseInfo, ...] = tuple(databases)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {len(self.databases)} databases>"

    @override
    async def acquire(self, region: Region, space: Space) -> "DatabaseInfo":
        from destack.store.postgres import pg_connection

        # find main database
        base_database: DatabaseInfo | None = None
        for database in self.databases:
            if database.region == region:
                base_database = database
                break
        if base_database is None:
            raise LookupError(f'no Database for "{region.slug}" in {self!r}')

        # create schema
        destack_schema_name = f"destack_{str(space.id).replace('-', '_')}"
        async with pg_connection(base_database) as conn:
            await conn.execute(f"CREATE SCHEMA IF NOT EXISTS {destack_schema_name}")
        database = DatabaseInfo(
            type=base_database.type,
            tenancy=Tenancy.SHARED,
            region=base_database.region,
            cell_name=base_database.cell_name,
            external_name=base_database.external_name,
            custom_schema_name=destack_schema_name,
            connection_url=base_database.connection_url,
        )
        return database

    @override
    async def resolve(
        self, region: Region, cell_name: str, external_name: str
    ) -> "DatabaseInfo | None":
        for database in self.databases:
            if (
                database.region == region
                and database.cell_name == cell_name
                and database.external_name == external_name
            ):
                return database
        return None

    @classmethod
    def parse(cls, provider_str: str) -> "StaticDatabaseProvider":
        """
        Parse a map string like:
        'eu-zurich/cell_1/external_name_a=postgresql://user:pass@host/db'
        'eu-zurich/cell_1/external_name_a=postgresql://user:pass@host/db;eu-zurich/cell_2/external_name_b=postgresql://user:pass@host/db'
        """
        from destack.language import DatabaseInfo

        databases = []
        for database_entry in provider_str.split(";"):
            database_entry = database_entry.strip()
            if not database_entry:
                continue
            if "=" not in database_entry:
                raise ValueError(f"invalid database: {database_entry}")
            location_part, database_url = database_entry.split("=", 1)
            if location_part.count("/") != 2:
                raise ValueError(f"invalid location: {location_part}")
            if "postgresql://" in database_url:
                type = DatabaseType.POSTGRES
            else:
                raise ValueError(f"unknown database: {database_url}")
            region_name, cell_name, external_name = location_part.split("/")
            region = REGION_BY_SLUG[region_name]
            database_info = DatabaseInfo(
                type=type,
                region=region,
                cell_name=cell_name,
                external_name=external_name,
                connection_url=database_url,
            )
            databases.append(database_info)

        return cls(databases)
