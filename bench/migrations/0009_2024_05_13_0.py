# This migration was automatically generated on 2024.05.13. Edit as needed.
import psycopg

ID = 9
VERSION = "2024.05.13.0"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_pause
    await cur.execute('DROP TABLE "bench_pause"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "parent_session_id"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "is_readonly"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "is_runtime"')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "status" smallint NOT NULL')
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_status" ON bench_session USING BTREE (status)'
    )

    # bench_run
    await cur.execute(
        'ALTER TABLE "bench_run" DROP CONSTRAINT IF EXISTS "bench_run_bench_check_one_parent", ADD CONSTRAINT "bench_run_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL) OR (parent_run_id IS NOT NULL))'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
