import typer
from rich.console import Console

from bench.cli.utils import async_to_sync_blocking

app = typer.Typer(short_help="test utilities")


@app.command("prune")
@async_to_sync_blocking
async def prune(prefix="test"):
    from bench.sql.client import pg_connection
    from bench.sql.engine import pg_select_raw, sqlstr
    from bench.system.utils.session import system_store_from_env

    """Prune all artifacts with the given prefix"""
    console = Console()
    global_store = system_store_from_env()
    async with pg_connection(global_store, autocommit=True) as conn:
        # select all databases
        results = await pg_select_raw(
            query=f"SELECT datname FROM pg_database WHERE datname LIKE '{prefix}_%'",
            cur=conn.cursor,
        )
        databases = [row["datname"] for row in results]
        # drop all databases
        for database in databases:
            console.print(f"DROP DATABASE {database}")
            await conn.execute(sqlstr(f'DROP DATABASE "{database}"'))
