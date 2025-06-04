import abc
from collections.abc import Sequence
from typing import TYPE_CHECKING, override

from bench.language.core.const import REGION_BY_SLUG, Region

if TYPE_CHECKING:
    from bench.language import CellInfo


class CellProvider(abc.ABC):
    """A provider of known Cells."""

    @abc.abstractmethod
    async def acquire(self, region: Region) -> "CellInfo":
        """Acquires a Cell for the given region."""
        ...

    @abc.abstractmethod
    async def resolve(self, region: Region, cell_name: str) -> "CellInfo | None":
        """Gets the cell info for the given region and cell name (error if none)."""
        ...

    async def resolve_or_error(self, region: Region, cell_name: str) -> "CellInfo":
        """Gets the cell info for the given region and cell name (error if none)."""
        cell_info = await self.resolve(region, cell_name)
        if cell_info is None:
            raise LookupError(f'no Cell for "{region.slug}/{cell_name}" in {self!r}')
        return cell_info


class StaticCellProvider(CellProvider):
    """A static CellProvider with a fixed list."""

    def __init__(self, cells: Sequence["CellInfo"]):
        self._cells: tuple[CellInfo, ...] = tuple(cells)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {len(self._cells)} cells>"

    @override
    async def acquire(self, region: Region) -> "CellInfo":
        for cell in self._cells:
            if cell.region == region:
                return cell
        raise LookupError(f'no Cell for "{region.slug}" in {self!r}')

    @override
    async def resolve(self, region: Region, cell_name: str) -> "CellInfo | None":
        for cell in self._cells:
            if cell.region == region and cell.name == cell_name:
                return cell
        return None

    @classmethod
    def parse(cls, cell_str: str) -> "StaticCellProvider":
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
            cell_info = CellInfo(region=region, name=cell_name, host=host_part)
            cells.append(cell_info)

        return cls(cells)
