# This migration was automatically generated on 2024.05.27. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.05.27.3"
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
    # bench_session
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_created_epoch" ON bench_session USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_package_id_created_epoch" ON bench_session USING BTREE (package_id, created_epoch)'
    )

    # bench_run
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_created_epoch" ON bench_run USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_package_id_created_epoch" ON bench_run USING BTREE (package_id, created_epoch)'
    )

    # bench_notification
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_created_epoch" ON bench_notification USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_package_id_created_epoch" ON bench_notification USING BTREE (package_id, created_epoch)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
