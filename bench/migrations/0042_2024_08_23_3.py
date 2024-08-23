# This migration was automatically generated on 2024.08.23. Edit as needed.
import psycopg

ID = 42
VERSION = "2024.08.23.3"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_port
    await cur.execute(
        """
        ALTER TABLE bench_port    
        ALTER COLUMN name SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
