from pathlib import Path

import structlog
import typer

from bench.language import Bench
from bench.system.utils import global_session

app = typer.Typer(short_help="local developer helpers")

server = typer.Typer()
logger = structlog.get_logger(__name__)


@server.command(name="imitate")
async def imitate_server(bench: str):
    """'Imitate' the env vars of a server for a Bench in .env.server"""
    async with global_session():
        bench: Bench = await Bench.get(slug=bench)
    logger.info("server.imitate", bench=bench)
    env_vars = {
        "SERVER_ID": "local",
        "BENCH_ID": str(bench.id),
        "LOCAL_PG_NAME": bench.pg_name,
        "LOCAL_PG_USERNAME": bench.pg_username,
        "LOCAL_PG_PASSWORD": bench.pg_password,
        "LOCAL_OS_NAME": bench.os_name,
        "LOCAL_OS_USERNAME": bench.os_username,
        "LOCAL_OS_PASSWORD": bench.os_password,
    }
    Path(".env.server").write_text("\n".join(f"{k}={v}" for k, v in env_vars.items()))
    logger.info(
        "server.imitate.done",
        bench=bench,
        **{k: v for k, v in env_vars.items() if "PASSWORD" not in k},
    )


@server.command(name="clear")
def clear_server():
    logger.info("server.clear")
    Path(".env.server").write_text("")
    logger.info("server.clear.done")


app.add_typer(server, name="server")
