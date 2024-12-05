# This migration was automatically generated on 2024.12.05. Edit as needed.
import psycopg

ID = 14
VERSION = "2024.12.05.2"
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
    # bench_message
    await cur.execute(
        """
        ALTER TABLE bench_message    
        ALTER COLUMN "origin_id" DROP NOT NULL,
        ALTER COLUMN "origin_ck" DROP NOT NULL,
        ALTER COLUMN "origin_type" DROP NOT NULL,
        ALTER COLUMN "origin_bench_id" DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
