# This migration was automatically generated on 2024.07.29. Edit as needed.
import psycopg

ID = 23
VERSION = "2024.07.29.0"
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
    # bench_file
    await cur.execute(
        """
        ALTER TABLE bench_file    
        ALTER COLUMN format SET DATA TYPE integer
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
