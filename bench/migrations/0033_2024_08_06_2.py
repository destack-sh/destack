# This migration was automatically generated on 2024.08.06. Edit as needed.
import psycopg

ID = 33
VERSION = "2024.08.06.2"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute(
        """
        ALTER TABLE bench_client    
        ALTER COLUMN seen_at DROP NOT NULL
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
