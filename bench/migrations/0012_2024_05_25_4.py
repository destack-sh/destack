# This migration was automatically generated on 2024.05.25. Edit as needed.
import psycopg

ID = 12
VERSION = "2024.05.25.4"
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
    # bench_session
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_created_at" ON bench_session USING BTREE (created_at)'
    )

    # bench_run
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_created_at" ON bench_run USING BTREE (created_at)'
    )

    # bench_signal
    await cur.execute(
        'CREATE INDEX "bench_signal_bench_idx_created_at" ON bench_signal USING BTREE (created_at)'
    )

    # bench_log
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_created_at" ON bench_log USING BTREE (created_at)'
    )

    # bench_notification
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_created_at" ON bench_notification USING BTREE (created_at)'
    )

    # bench_message
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_created_at" ON bench_message USING BTREE (created_at)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    # bench_message
    await cur.execute('DROP INDEX "bench_message_bench_idx_created_at"')

    # bench_notification
    await cur.execute('DROP INDEX "bench_notification_bench_idx_created_at"')

    # bench_log
    await cur.execute('DROP INDEX "bench_log_bench_idx_created_at"')

    # bench_signal
    await cur.execute('DROP INDEX "bench_signal_bench_idx_created_at"')

    # bench_run
    await cur.execute('DROP INDEX "bench_run_bench_idx_created_at"')

    # bench_session
    await cur.execute('DROP INDEX "bench_session_bench_idx_created_at"')
