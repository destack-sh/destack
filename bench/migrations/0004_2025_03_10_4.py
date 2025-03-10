# This migration was automatically generated on 2025.03.10. Edit as needed.
import psycopg

ID = 4
VERSION = "2025.03.10.4"
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
    # bench_run_span
    await cur.execute(
        """
        ALTER TABLE "bench_run_span"    
        ADD COLUMN "flow_id" uuid,
        ADD COLUMN "kit_id" uuid,
        ADD COLUMN "action_id" uuid,
        ADD COLUMN "link_id" uuid,
        ADD COLUMN "plan_id" uuid,
        ADD COLUMN "plan_ck" uuid,
        ADD COLUMN "task_id" uuid,
        ADD COLUMN "task_ck" uuid,
        ADD COLUMN "trigger_id" uuid,
        ADD COLUMN "trigger_key" varchar
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
