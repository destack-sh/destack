# This migration was automatically generated on 2025.05.11. Edit as needed.
import psycopg

ID = 25
VERSION = "2025.05.11.0"
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
    # bench_trigger
    await cur.execute('DROP TABLE "bench_trigger"')

    # bench_plan
    await cur.execute('DROP TABLE "bench_plan"')

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "plan_ck",
        DROP COLUMN "plan_id",
        DROP COLUMN "target_base_id",
        DROP COLUMN "target_ck",
        DROP COLUMN "target_id",
        DROP COLUMN "target_type",
        DROP COLUMN "trigger_id"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "plan_ck",
        DROP COLUMN "plan_id",
        DROP COLUMN "target_base_id",
        DROP COLUMN "target_ck",
        DROP COLUMN "target_id",
        DROP COLUMN "target_type",
        DROP COLUMN "trigger_id"
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        DROP COLUMN "run_base_id",
        DROP COLUMN "run_bench_id",
        DROP COLUMN "run_id"
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE "bench_role"    
        ADD COLUMN "parent_type" smallint
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
