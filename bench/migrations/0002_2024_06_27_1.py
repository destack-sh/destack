# This migration was automatically generated on 2024.06.27. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.06.27.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "roles_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "roles_ck" uuid[]')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "roles_bench_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "identity_id" uuid')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "identity_ck" uuid')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "identity_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "policies" jsonb[]')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
