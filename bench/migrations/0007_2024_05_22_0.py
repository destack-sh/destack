# This migration was automatically generated on 2024.05.22. Edit as needed.
import psycopg

ID = 7
VERSION = "2024.05.22.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "bumped_at"')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "current_version" varchar')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "machine_bench_id" uuid')
    await cur.execute(
        'ALTER TABLE "bench_client" ADD COLUMN "machine_id" uuid REFERENCES bench_machine ON DELETE SET NULL'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
