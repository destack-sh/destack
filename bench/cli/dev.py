from pathlib import Path

import structlog
import typer

from bench.language import Bench
from bench.server.utils import detached_session

app = typer.Typer(short_help="local developer helpers")

worker = typer.Typer()
logger = structlog.get_logger(__name__)


@worker.command(name="imitate")
async def imitate_worker(bench: str):
    """'Imitate' the env vars of a worker for a Bench in .env.worker"""
    async with detached_session(read_only=True):
        bench: Bench = await Bench.get(slug=bench)
    logger.info("worker.imitate", bench=bench)
    env_vars = {
        "WORKER_SET_ID": str(bench.worker_set.id),
        "WORKER_ID": "local",
        "BENCH_ID": str(bench.id),
        "PACKAGE_ID": str(bench.head_id),
        "LOCAL_PG_NAME": bench.pg_name,
        "LOCAL_PG_USERNAME": bench.pg_username,
        "LOCAL_PG_PASSWORD": bench.pg_password,
        "LOCAL_OS_NAME": bench.os_name,
        "LOCAL_OS_USERNAME": bench.os_username,
        "LOCAL_OS_PASSWORD": bench.os_password,
    }
    Path(".env.worker").write_text("\n".join(f"{k}={v}" for k, v in env_vars.items()))
    logger.info(
        "worker.imitate.done",
        bench=bench,
        **{k: v for k, v in env_vars.items() if "PASSWORD" not in k},
    )


@worker.command(name="clear")
def clear_worker():
    logger.info("worker.clear")
    Path(".env.worker").write_text("")
    logger.info("worker.clear.done")


app.add_typer(worker, name="worker")
