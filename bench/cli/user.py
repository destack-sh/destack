import structlog
import typer

from bench.language import NodeArea, User, UserStatus
from bench.utils.func import generate_salt
from bench.utils.oracle import REAL_ORACLE

from .utils import async_to_sync

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command("unwaitlist", help="check whether the current Bench state is properly migrated")
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


@app.command("setpassword", help="(re)set a User's password")
@async_to_sync
async def set_password(user_slug: str, new_password: str):
    from bench.system import (
        SALT_LENGTH,
        global_session,
        global_store_from_env,
        hash_password,
        pg_engine_from_store,
    )

    global_store = global_store_from_env()
    global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
    async with global_session(global_store, (global_pg_engine,), REAL_ORACLE, epoch=0) as session:
        user = await User.get(slug=user_slug)
        session._track(user)
        user.password_salt = generate_salt(SALT_LENGTH)
        user.password_hash = hash_password(new_password, user.password_salt)
        logger.info("user.set_password", user=user)
        await session.commit()
