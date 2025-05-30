import abc
from typing import TYPE_CHECKING, Sequence, override

from .const import REGION_BY_SLUG, Region

if TYPE_CHECKING:
    from bench.language import CellInfo, DatabaseInfo


#
# Cells
#


class CellRegistry(abc.ABC):
    @abc.abstractmethod
    def get(self, region: Region, cell_name: str) -> "CellInfo | None":
        """Gets the cell info for the given region and cell name (error if none)."""
        ...

    def get_or_error(self, region: Region, cell_name: str) -> "CellInfo":
        """Gets the cell info for the given region and cell name (error if none)."""
        cell_info = self.get(region, cell_name)
        if cell_info is None:
            raise LookupError(f"no cell info for {region.bench_name} in {self!r}")
        return cell_info


class StaticCellRegistry(CellRegistry):
    def __init__(self, cells: Sequence["CellInfo"]):
        self._cells: tuple[CellInfo, ...] = tuple(cells)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {len(self._cells)} cells>"

    @override
    def get(self, region: Region, cell_name: str) -> "CellInfo | None":
        raise NotImplementedError

    @classmethod
    def parse(cls, cell_str: str) -> "StaticCellRegistry":
        """
        Parse a cell map string like:
        'eu-zurich/cell_1=localcell:60061/8080'
        'eu-frankfurt/cell_1=aws-eu-frankfurt-x7ahko.justbench.com'
        'eu-frankfurt/cell_2=aws-eu-frankfurt-21uza3.justbench.com'
        """
        from bench.language import CellInfo

        cells = []
        for cell_entry in cell_str.split(";"):
            cell_entry = cell_entry.strip()
            if not cell_entry:
                continue
            if "=" not in cell_entry:
                raise ValueError(f"invalid cell entry format: {cell_entry}")
            location_part, host_part = cell_entry.split("=", 1)
            if "/" not in location_part:
                raise ValueError(f"invalid location format: {location_part}")
            region_name, cell_name = location_part.split("/", 1)
            region = REGION_BY_SLUG[region_name]
            cell_info = CellInfo(region=region, cell_name=cell_name, host=host_part)
            cells.append(cell_info)

        return cls(cells)


#
# Databases
#


class DatabaseRegistry(abc.ABC):
    @abc.abstractmethod
    def get(self, region: Region, cell_name: str, external_id: str) -> "DatabaseInfo | None":
        """Gets the Database for the given region (error if none)."""
        ...

    def get_or_error(self, region: Region, cell_name: str, external_id: str) -> "DatabaseInfo":
        """Gets the Database for the given region (error if none)."""
        database_info = self.get(region, cell_name, external_id)
        if database_info is None:
            raise LookupError(f"no database info for {region.bench_name} in {self!r}")
        return database_info


class StaticDatabaseRegistry(DatabaseRegistry):
    """A static database registry backed by a sequence of DatabaseInfo."""

    def __init__(self, databases: Sequence["DatabaseInfo"]):
        self.databases: tuple[DatabaseInfo, ...] = tuple(databases)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {len(self.databases)} databases>"

    @override
    def get(self, region: Region, cell_name: str, external_id: str) -> "DatabaseInfo | None":
        raise NotImplementedError

    @classmethod
    def parse(cls, registry_str: str) -> "StaticDatabaseRegistry":
        """
        Parse a map string like:
        'eu-zurich/cell_1/external_id_a=postgresql://user:pass@host/db'
        'eu-zurich/cell_1/external_id_a=postgresql://user:pass@host/db;eu-zurich/cell_2/external_id_b=postgresql://user:pass@host/db'
        """
        from bench.language import DatabaseInfo

        databases = []
        for database_entry in registry_str.split(";"):
            database_entry = database_entry.strip()
            if not database_entry:
                continue
            if "=" not in database_entry:
                raise ValueError(f"invalid database entry format: {database_entry}")
            location_part, database_url = database_entry.split("=", 1)
            if location_part.count("/") != 2:
                raise ValueError(f"invalid location format: {location_part}")
            region_name, cell_name, external_id = location_part.split("/")
            region = REGION_BY_SLUG[region_name]
            database_info = DatabaseInfo(
                region=region,
                cell_name=cell_name,
                external_id=external_id,
                sql_url=database_url,
            )
            databases.append(database_info)

        return cls(databases)
