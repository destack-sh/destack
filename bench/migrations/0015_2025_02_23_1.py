# This migration was automatically generated on 2025.02.23. Edit as needed.
import psycopg

ID = 15
VERSION = "2025.02.23.1"
HAS_GLOBAL = False
HAS_REGIONAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_page
    await cur.execute('ALTER TABLE "bench_page" ADD COLUMN "thread_id" uuid')

    # bench_choice
    await cur.execute('ALTER TABLE "bench_choice" ADD COLUMN "thread_id" uuid')

    # bench_class
    await cur.execute('ALTER TABLE "bench_class" ADD COLUMN "thread_id" uuid')

    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "thread_id" uuid')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "thread_id" uuid')

    # bench_database
    await cur.execute('ALTER TABLE "bench_database" ADD COLUMN "thread_id" uuid')

    # bench_channel
    await cur.execute('ALTER TABLE "bench_channel" ADD COLUMN "thread_id" uuid')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "thread_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
