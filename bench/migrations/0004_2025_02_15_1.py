# This migration was automatically generated on 2025.02.15. Edit as needed.
import psycopg

ID = 4
VERSION = "2025.02.15.1"
HAS_GLOBAL = False
HAS_REGIONAL = False
HAS_LOCAL = True


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
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "action_bench_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "flow_bench_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "pipe_bench_id"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_base_bench_id" uuid')

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN "page_id" SET NOT NULL,
        ALTER COLUMN "page_ck" SET NOT NULL,
        ALTER COLUMN "page_bench_id" SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
