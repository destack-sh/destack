"""Python legacy alias package for the canonical destack client package."""

from destack import BACKEND, VERSION, DestackClient, create_client

__all__ = ["BACKEND", "VERSION", "DestackClient", "create_client"]
