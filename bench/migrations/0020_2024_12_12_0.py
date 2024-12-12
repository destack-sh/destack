# This migration was automatically generated on 2024.12.12. Edit as needed.
import psycopg

ID = 20
VERSION = "2024.12.12.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ALTER COLUMN "target_version" SET NOT NULL
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ALTER COLUMN "target_version" SET NOT NULL
    """
    )

    # bench_machine
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN "target_version" SET NOT NULL,
    ALTER COLUMN "target_version" DROP DEFAULT
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
