# This migration was automatically generated on 2025.02.05. Edit as needed.
import psycopg

ID = 3
VERSION = "2025.02.05.0"
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
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "class__bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "class__ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "class__id"')

    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "run_options" jsonb')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "clazz_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "clazz_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "clazz_bench_id" uuid')

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE bench_thread    
        ALTER COLUMN "status" SET DEFAULT 10
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
