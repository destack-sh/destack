# This migration was automatically generated on 2024.06.04. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.06.04.1"
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
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "old_node_secret_packed" bytea')
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "new_node_secret_packed" bytea')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
