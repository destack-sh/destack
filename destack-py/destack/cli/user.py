import structlog
import typer

from destack.language import Session, StoreType, User
from destack.utils.func import generate_salt

from .utils import async_to_sync

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command("setpassword", help="(re)set a User's password")
@async_to_sync
async def set_password(user_slug: str, new_password: str):
    from destack.destack import SALT_LENGTH, hash_password
    from destack.sharding import get_global_database_from_env
    from destack.store import PostgresStore

    global_database = get_global_database_from_env()
    store = PostgresStore(database=global_database, types=(StoreType.GLOBAL_ENTITY,))
    async with Session(store=store) as session:
        user = await User.get(where=User.property("slug").eq(user_slug)).execute_one()
        user.password_salt = generate_salt(SALT_LENGTH)
        user.password_hash = hash_password(new_password, user.password_salt)
        logger.info("user.set_password", user=user)
        await session.commit()
