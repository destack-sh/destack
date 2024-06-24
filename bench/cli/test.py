import typer
from rich.console import Console

from bench.cli.utils import async_to_sync_blocking
from bench.sql.engine import pg_select_raw, sqlstr
from bench.system.core import (
    global_pg_cursor,
    global_store_from_env,
)

app = typer.Typer(short_help="test utilities")


@app.command("prune")
@async_to_sync_blocking
async def prune(prefix="test"):
    """Prune all artifacts with the given prefix"""
    console = Console()
    global_store = global_store_from_env()
    async with global_pg_cursor(global_store, autocommit=True) as cur:
        # select all databases
        results = await pg_select_raw(
            query=f"SELECT datname FROM pg_database WHERE datname LIKE '{prefix}_%'", cur=cur
        )
        databases = [row["datname"] for row in results]
        # drop all databases
        for database in databases:
            console.print(f"DROP DATABASE {database}")
            await cur.execute(sqlstr(f'DROP DATABASE "{database}"'))
