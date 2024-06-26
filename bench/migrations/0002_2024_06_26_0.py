# This migration was automatically generated on 2024.06.26. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.06.26.0"
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
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "secret_value_packed"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "text"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "title"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "value_packed"')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "change_id" uuid')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "nodes_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "nodes_ck" uuid[]')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "nodes_type" smallint[]')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "nodes_base_ck" uuid[]')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
