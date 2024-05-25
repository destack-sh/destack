# This migration was automatically generated on 2024.05.25. Edit as needed.
import psycopg

ID = 9
VERSION = "2024.05.25.0"
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
    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "event"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "logger"')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "epoch" bigint')
    await cur.execute('CREATE INDEX "bench_log_bench_idx_epoch" ON bench_log USING BTREE (epoch)')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
