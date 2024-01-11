import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking
from bench.language.node import Bench
from bench.server.session import detached_session
from bench.sql.client import async_pg_cursor
from bench.sql.engine import map_node_type_to_pg_table
from bench.sql.migration import introspect_tables_from_pg
from bench.utils.utils import format_python

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="pg management")


@app.command()
@_async_to_sync_blocking
async def introspect(bench: str = None):
    """Introspect the current schema of the Postgres instance."""
    if bench is not None:
        async with detached_session(read_only=True):
            bench = Bench.get(slug=bench)
            local_pg_name = bench.pg_name
    else:
        local_pg_name = None

    # introspect
    async with async_pg_cursor(local_pg_name=local_pg_name) as cur:
        tables = await introspect_tables_from_pg(
            cur, include_columns=True, include_indexes=True, include_constraints=True
        )

    # generate schema
    chunks: list[str] = []
    for table in tables:
        const_name = f"{table.name}_TABLE".upper()
        if const_name.startswith("BENCH_"):
            const_name = const_name[6:]
        table_def = f"{const_name} = {table.source_repr()}"
        chunks.append(table_def)
    source = "\n\n".join(chunks)
    source = format_python(source)
    print(source)
