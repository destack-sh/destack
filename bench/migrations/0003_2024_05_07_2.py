# This migration was automatically generated on 2024.05.07. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.05.07.2"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "condition" jsonb')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "node_type" smallint')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "type" smallint NOT NULL')

    # bench_trigger
    await cur.execute(
        'ALTER TABLE "bench_trigger" ADD COLUMN "parent_step_id" uuid REFERENCES bench_step ON DELETE CASCADE'
    )
    await cur.execute(
        'ALTER TABLE "bench_trigger" DROP CONSTRAINT IF EXISTS "bench_trigger_bench_check_one_parent", ADD CONSTRAINT "bench_trigger_bench_check_one_parent" CHECK ((parent_block_id IS NOT NULL) OR (parent_step_id IS NOT NULL))'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_session
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "client_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "user_id" uuid')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "parent_package_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "client_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "client_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "user_id" uuid')
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN session_id DROP NOT NULL,
        ALTER COLUMN session_ck DROP NOT NULL,
        ALTER COLUMN session_bench_id DROP NOT NULL
    """
    )
    await cur.execute(
        'ALTER TABLE "bench_run" DROP CONSTRAINT IF EXISTS "bench_run_bench_check_one_parent", ADD CONSTRAINT "bench_run_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL) OR (parent_session_id IS NOT NULL) OR (parent_run_id IS NOT NULL))'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
