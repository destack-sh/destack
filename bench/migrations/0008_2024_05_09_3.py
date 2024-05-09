# This migration was automatically generated on 2024.05.09. Edit as needed.
import psycopg

ID = 8
VERSION = "2024.05.09.3"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_message
    await cur.execute(
        'ALTER TABLE "bench_message" ADD COLUMN "parent_message_id" uuid REFERENCES bench_message ON DELETE CASCADE'
    )
    await cur.execute(
        'ALTER TABLE "bench_message" DROP CONSTRAINT IF EXISTS "bench_message_bench_check_one_parent", ADD CONSTRAINT "bench_message_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL) OR (parent_block_id IS NOT NULL) OR (parent_message_id IS NOT NULL))'
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
