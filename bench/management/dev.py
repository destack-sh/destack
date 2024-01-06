from pathlib import Path

import typer

app = typer.Typer()

worker = typer.Typer()


@worker.command(name="imitate")
async def imitate_worker(bench: str):
    """'Imitate' the env vars of a worker for a Bench in .env.worker"""
    bench = await get_bench(bench)
    env_vars = {
        "WORKER_SET_ID": str(bench.worker_set.id),
        "WORKER_ID": "local",
        "WORKER_BENCH_ID": str(bench.id),
        "WORKER_MODULE_ID": str(bench.head_id),
        "LOCAL_PG_NAME": bench.pg_name,
        "LOCAL_PG_USERNAME": bench.pg_username,
        "LOCAL_PG_PASSWORD": bench.pg_password,
        "LOCAL_OS_NAME": bench.os_name,
        "LOCAL_OS_USERNAME": bench.os_username,
        "LOCAL_OS_PASSWORD": bench.os_password,
    }
    Path(".env.worker").write_text("\n".join(f"{k}={v}" for k, v in env_vars.items()))
    typer.echo(f"patched .env.worker for {bench}")


@worker.command(name="clear")
def clear_worker():
    Path(".env.worker").write_text("")
    typer.echo("cleared .env.worker")


app.add_typer(worker, name="worker")
