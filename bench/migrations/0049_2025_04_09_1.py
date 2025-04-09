# This migration was automatically generated on 2025.04.09. Edit as needed.
import psycopg

ID = 49
VERSION = "2025.04.09.1"
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
    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "message_base_id",
        DROP COLUMN "message_id"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "message_base_id",
        DROP COLUMN "message_id"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "target_id" uuid,
        ADD COLUMN "target_ck" uuid,
        ADD COLUMN "target_type" smallint,
        ADD COLUMN "target_base_id" uuid
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "target_id" uuid,
        ADD COLUMN "target_ck" uuid,
        ADD COLUMN "target_type" smallint,
        ADD COLUMN "target_base_id" uuid
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
