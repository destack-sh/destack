import abc
from collections.abc import Sequence
from typing import TYPE_CHECKING, override

from destack.language import REGION_BY_SLUG, Region, Space

if TYPE_CHECKING:
    from destack.language import GalaxyInfo


class GalaxyProvider(abc.ABC):
    """A provider of known Galaxys."""

    @abc.abstractmethod
    async def acquire(self, region: Region, space: Space) -> "GalaxyInfo":
        """Acquires a new (shared) Galaxy for the Space."""
        ...

    @abc.abstractmethod
    async def resolve(self, region: Region, galaxy_name: str) -> "GalaxyInfo | None":
        """Gets the galaxy info for the given region and galaxy name (error if none)."""
        ...

    async def resolve_or_error(self, region: Region, galaxy_name: str) -> "GalaxyInfo":
        """Gets the galaxy info for the given region and galaxy name (error if none)."""
        galaxy_info = await self.resolve(region, galaxy_name)
        if galaxy_info is None:
            raise LookupError(f'no Galaxy for "{region.slug}/{galaxy_name}" in {self!r}')
        return galaxy_info


class StaticGalaxyProvider(GalaxyProvider):
    """A static GalaxyProvider with a fixed list."""

    def __init__(self, galaxys: Sequence["GalaxyInfo"]):
        self._galaxys: tuple[GalaxyInfo, ...] = tuple(galaxys)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {len(self._galaxys)} galaxys>"

    @override
    async def acquire(self, region: Region, space: Space) -> "GalaxyInfo":
        for galaxy in self._galaxys:
            if galaxy.region == region:
                return galaxy
        raise LookupError(f'no Galaxy for "{region.slug}" in {self!r}')

    @override
    async def resolve(self, region: Region, galaxy_name: str) -> "GalaxyInfo | None":
        for galaxy in self._galaxys:
            if galaxy.region == region and galaxy.name == galaxy_name:
                return galaxy
        return None

    @classmethod
    def parse(cls, galaxy_str: str) -> "StaticGalaxyProvider":
        """
        Parse a galaxy map string like:
        'eu-zurich/galaxy_1=localgalaxy:60061/8080'
        'eu-frankfurt/galaxy_1=aws-eu-frankfurt-x7ahko.justdestack.com'
        'eu-frankfurt/galaxy_2=aws-eu-frankfurt-21uza3.justdestack.com'
        """
        from destack.language import GalaxyInfo

        galaxys = []
        for galaxy_entry in galaxy_str.split(";"):
            galaxy_entry = galaxy_entry.strip()
            if not galaxy_entry:
                continue
            if "=" not in galaxy_entry:
                raise ValueError(f"invalid galaxy entry format: {galaxy_entry}")
            location_part, host_part = galaxy_entry.split("=", 1)
            if "/" not in location_part:
                raise ValueError(f"invalid location format: {location_part}")
            region_name, galaxy_name = location_part.split("/", 1)
            region = REGION_BY_SLUG[region_name]
            galaxy_info = GalaxyInfo(region=region, name=galaxy_name, host=host_part)
            galaxys.append(galaxy_info)

        return cls(galaxys)
