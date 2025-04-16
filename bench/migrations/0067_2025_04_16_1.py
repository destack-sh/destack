# This migration was automatically generated on 2025.04.16. Edit as needed.
import psycopg

ID = 67
VERSION = "2025.04.16.1"
HAS_GLOBAL = False
HAS_REGIONAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        DROP COLUMN "clazz_bench_id",
        DROP COLUMN "clazz_id"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "clazz_bench_id",
        DROP COLUMN "clazz_id"
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "runnable_id" uuid,
        ADD COLUMN "runnable_ck" uuid,
        ADD COLUMN "runnable_type" smallint,
        ADD COLUMN "runnable_bench_id" uuid
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
