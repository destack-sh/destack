import structlog
import typer

from bench.language import Area, Session, User
from bench.utils.func import generate_salt

from .utils import async_to_sync

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command("setpassword", help="(re)set a User's password")
@async_to_sync
async def set_password(user_slug: str, new_password: str):
    from bench.sharding import get_global_database_from_env
    from bench.store import DatabaseStore
    from bench.supervisor import SALT_LENGTH, hash_password

    global_database = get_global_database_from_env()
    store = DatabaseStore(database=global_database, area=Area.GLOBAL_DATABASE)
    async with Session(store=store) as session:
        user = await User.get(where=User.property("slug").eq(user_slug)).execute_one()
        user.password_salt = generate_salt(SALT_LENGTH)
        user.password_hash = hash_password(new_password, user.password_salt)
        logger.info("user.set_password", user=user)
        await session.commit()
