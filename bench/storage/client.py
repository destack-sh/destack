from sqlalchemy.ext.asyncio import AsyncEngine, create_async_engine

from bench.utils.utils import get_from_env

PG_HOST = get_from_env("LOCAL_PG_HOST", default="localhost", alt="USER_PG_HOST")
PG_NAME = get_from_env("LOCAL_PG_NAME", optional=True)
PG_PORT = get_from_env("LOCAL_PG_PORT", default=9200, type_cast=int, alt="USER_PG_PORT")
PG_USERNAME = get_from_env("LOCAL_PG_USERNAME", default="admin", alt="USER_PG_USERNAME")
PG_PASSWORD = get_from_env("LOCAL_PG_PASSWORD", default="admin", alt="USER_PG_PASSWORD")

_PG_ENGINES: dict[str, AsyncEngine] = {}


def create_async_pg_engine_to(db_name: str):
    if db_name not in _PG_ENGINES:
        engine = create_async_engine(
            f"postgresql://{PG_USERNAME}:{PG_PASSWORD}@{PG_HOST}:{PG_PORT}/{db_name}",
            future=True,
        )
        _PG_ENGINES[db_name] = engine
    return _PG_ENGINES[db_name]


if PG_NAME:
    pg_engine = create_async_pg_engine_to(PG_NAME)
else:
    pg_engine = None
