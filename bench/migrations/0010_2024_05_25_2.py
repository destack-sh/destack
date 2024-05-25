# This migration was automatically generated on 2024.05.25. Edit as needed.
import psycopg

ID = 10
VERSION = "2024.05.25.2"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    pass


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "node_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "node_bench_id"')

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN root_id SET NOT NULL,
        ALTER COLUMN root_ck SET NOT NULL,
        ALTER COLUMN root_base_ck SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN root_base_ck DROP NOT NULL,
        ALTER COLUMN root_ck DROP NOT NULL,
        ALTER COLUMN root_id DROP NOT NULL
    """
    )

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "node_base_bench_id" uuid')
