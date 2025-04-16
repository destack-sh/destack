# This migration was automatically generated on 2025.04.16. Edit as needed.
import psycopg

ID = 68
VERSION = "2025.04.16.2"
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
    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "is_manual",
        DROP COLUMN "target_bench_id",
        DROP COLUMN "target_ck",
        DROP COLUMN "target_id",
        DROP COLUMN "target_type",
        DROP COLUMN "tool_bench_id",
        DROP COLUMN "tool_ck",
        DROP COLUMN "tool_id",
        DROP COLUMN "tool_type",
        DROP COLUMN "value_packed",
        ADD COLUMN "type" smallint NOT NULL DEFAULT 10
    """
    )

    # Remove the default after setting it
    await cur.execute(
        """
        ALTER TABLE "bench_task"
        ALTER COLUMN "type" DROP DEFAULT
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
