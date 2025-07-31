import os
import subprocess
from typing import TYPE_CHECKING

import typer

from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    from destack.language import GraphKey, Region

logger = get_logger(__name__)
tracer = get_tracer(__name__)


def run_shell_sync(cmd: str, check=True, **kwargs):
    """Executes a shell command in a subprocess."""
    cwd = os.getcwd()
    logger.debug("shell", cmd=cmd, cwd=cwd, check=check, **kwargs)
    subprocess.run(cmd, shell=True, check=check, **kwargs)


def parse_region(region: "str | Region") -> "Region":
    """Parse a Region from a string."""
    from destack.language.core import REGION_BY_SLUG, Region

    if isinstance(region, Region):
        return region

    region = region.lower()
    try:
        if region in REGION_BY_SLUG:
            # try by slug
            return REGION_BY_SLUG[region]
        elif region in Region.__members__:
            # try by name
            return Region[region]
        else:
            # try by value
            return Region(int(region))
    except (TypeError, ValueError) as e:
        raise typer.BadParameter(
            f"invalid region: '{region}' (expected: {'|'.join(r.slug for r in Region)})"
        ) from e


def parse_store_key(store_key: "str | GraphKey") -> "GraphKey":
    """Parse a NodeArea from a string."""
    from destack.language.core import GraphKey

    if isinstance(store_key, GraphKey):
        return store_key

    store_key = store_key.upper()
    try:
        if store_key in GraphKey.__members__:
            # try by name
            return GraphKey[store_key]
        else:
            # try by value
            return GraphKey(int(store_key))
    except (TypeError, ValueError) as e:
        raise typer.BadParameter(
            f"invalid store key: '{store_key}' (expected: {'|'.join(a.name.lower() for a in GraphKey)})"
        ) from e
