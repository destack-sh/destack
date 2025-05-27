import structlog
import typer

from bench.language import NodeArea, User
from bench.utils.func import generate_salt
from bench.utils.oracle import REAL_ORACLE

from .utils import async_to_sync

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command("setpassword", help="(re)set a User's password")
@async_to_sync
async def set_password(user_slug: str, new_password: str):
    from bench.system import (
        SALT_LENGTH,
        get_global_database_from_env,
        global_session,
        hash_password,
        pg_engine_from_database,
    )

    global_database = get_global_database_from_env()
    global_pg_engine = pg_engine_from_database(
        "pg-global", global_database, NodeArea.GLOBAL_POSTGRES
    )
    async with global_session(global_database, (global_pg_engine,), oracle=REAL_ORACLE) as session:
        user = await User.search(where=User.property("slug").eq(user_slug)).execute_one()
        user.password_salt = generate_salt(SALT_LENGTH)
        user.password_hash = hash_password(new_password, user.password_salt)
        logger.info("user.set_password", user=user)
        await session.commit()
