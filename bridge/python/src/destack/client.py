from __future__ import annotations

from importlib.metadata import PackageNotFoundError, version as package_version
from pathlib import Path


def _version_from_local_pyproject() -> str | None:
    pyproject = Path(__file__).resolve().parents[2] / "pyproject.toml"
    if not pyproject.exists():
        return None

    text = pyproject.read_text(encoding="utf8")
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("version = "):
            continue

        return stripped.split("=", 1)[1].strip().strip('"').strip("'")

    return None


def _resolve_version() -> str:
    local_version = _version_from_local_pyproject()
    if local_version is not None:
        return local_version

    try:
        return package_version("destack")
    except PackageNotFoundError as error:
        raise RuntimeError("failed to resolve destack package version") from error


VERSION = _resolve_version()
BACKEND = "python"


class DestackClient:
    """A client for Destack."""

    backend = BACKEND

    def version(self) -> str:
        """Return the package version."""

        return VERSION


def create_client() -> DestackClient:
    """Create a client instance."""

    return DestackClient()
