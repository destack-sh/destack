from __future__ import annotations

import ctypes
import os
import platform
from importlib.metadata import PackageNotFoundError, version as package_version
from typing import Final

FALLBACK_VERSION: Final = "0.55.4"
BACKEND = "python"


class _CapiBindings:
    def __init__(self, library: ctypes.CDLL) -> None:
        self._abi_version = library.destack_capi_abi_version
        self._abi_version.restype = ctypes.c_uint32

        self._version = library.destack_capi_version
        self._version.restype = ctypes.c_char_p

    @classmethod
    def try_load(cls) -> _CapiBindings | None:
        explicit_path = os.environ.get("DESTACK_CAPI_LIB")
        candidates = []
        if explicit_path:
            candidates.append(explicit_path)

        if os.name == "nt":
            candidates.append("destack_capi.dll")
        elif platform.system() == "Darwin":
            candidates.append("libdestack_capi.dylib")
        else:
            candidates.append("libdestack_capi.so")

        for candidate in candidates:
            try:
                library = ctypes.CDLL(candidate)
            except OSError:
                continue

            try:
                return cls(library)
            except AttributeError:
                continue

        return None

    def capi_abi_version(self) -> int:
        return int(self._abi_version())

    def capi_version(self) -> str:
        value = self._version()
        if value is None:
            return FALLBACK_VERSION

        text = value.decode("utf8")
        if text == "":
            return FALLBACK_VERSION

        return text


def _resolve_version() -> str:
    try:
        return package_version("destack")
    except PackageNotFoundError:
        return FALLBACK_VERSION


VERSION = _resolve_version()
_CAPI = _CapiBindings.try_load()


class DestackClient:
    """A client for Destack."""

    backend = BACKEND

    def version(self) -> str:
        """Return the package version."""

        if _CAPI is None:
            return VERSION

        return _CAPI.capi_version()

    def capi_abi_version(self) -> int:
        """Return the loaded capi abi version."""

        if _CAPI is None:
            return 0

        return _CAPI.capi_abi_version()


def create_client() -> DestackClient:
    """Create a client instance."""

    return DestackClient()
