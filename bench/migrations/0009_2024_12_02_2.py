# This migration was automatically generated on 2024.12.02. Edit as needed.
import psycopg

ID = 9
VERSION = "2024.12.02.2"
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
        ALTER COLUMN "block_id" DROP NOT NULL,
        ALTER COLUMN "block_ck" DROP NOT NULL,
        ALTER COLUMN "block_bench_id" DROP NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
