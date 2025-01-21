import structlog
import typer

from bench.cli.utils import async_to_sync
from bench.language import NodeArea, User, UserStatus
from bench.utils.oracle import REAL_ORACLE

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@async_to_sync
async def unwaitlist(user_slug: str):
    from bench.system import (
        global_session,
        global_store_from_env,
        pg_engine_from_store,
    )

    global_store = global_store_from_env()
    global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
    async with global_session(global_store, (global_pg_engine,), REAL_ORACLE, epoch=0) as session:
        user = await User.get(slug=user_slug)
        if user.status != UserStatus.WAITLISTED:
            raise ValueError(f"{user!r} is not in the waitlist")
        user.status = UserStatus.REGISTERED
        logger.info("user.unwaitlist", user=user)
        await session.commit()
