# This migration was automatically generated on 2024.06.07. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.06.07.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "builtin_base"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "run"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "run"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "value_type" jsonb')
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "run_options" jsonb')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "run_options" jsonb')

    # bench_query
    await cur.execute(
        """
        ALTER TABLE bench_query    
        ALTER COLUMN name SET NOT NULL
    """
    )

    # bench_step
    await cur.execute(
        """
        ALTER TABLE bench_step    
        ALTER COLUMN type DROP DEFAULT,
        ALTER COLUMN name SET NOT NULL
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_signal
    await cur.execute(
        """
        ALTER TABLE bench_signal    
        ALTER COLUMN type_id SET NOT NULL,
        ALTER COLUMN type_ck SET NOT NULL,
        ALTER COLUMN type_bench_id SET NOT NULL
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        ALTER COLUMN type_id SET NOT NULL,
        ALTER COLUMN type_ck SET NOT NULL,
        ALTER COLUMN type_bench_id SET NOT NULL
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
