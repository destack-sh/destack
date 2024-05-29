# This migration was automatically generated on 2024.05.29. Edit as needed.
import psycopg

ID = 4
VERSION = "2024.05.29.0"
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
    # bench_message
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_created_epoch" ON bench_message USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_package_id_created_epoch" ON bench_message USING BTREE (package_id, created_epoch)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
