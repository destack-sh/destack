from sqlalchemy.ext.asyncio import AsyncEngine, create_async_engine

from bench.utils.utils import get_from_env

PG_HOST = get_from_env("LOCAL_PG_HOST", alt="USER_PG_HOST")
PG_NAME = get_from_env("LOCAL_PG_NAME", optional=True)
PG_PORT = get_from_env("LOCAL_PG_PORT", default=5432, type_cast=int, alt="USER_PG_PORT")
PG_USERNAME = get_from_env("LOCAL_PG_USERNAME", alt="USER_PG_USERNAME")
PG_PASSWORD = get_from_env("LOCAL_PG_PASSWORD", alt="USER_PG_PASSWORD")

_PG_ENGINES: dict[str, AsyncEngine] = {}


def create_async_pg_engine_to(pg_name: str):
    if pg_name not in _PG_ENGINES:
        engine = create_async_engine(
            f"postgresql://{PG_USERNAME}:{PG_PASSWORD}@{PG_HOST}:{PG_PORT}/{pg_name}",
            future=True,
        )
        _PG_ENGINES[pg_name] = engine
    return _PG_ENGINES[pg_name]


if PG_NAME:
    pg_engine = create_async_pg_engine_to(PG_NAME)
else:
    pg_engine = None
