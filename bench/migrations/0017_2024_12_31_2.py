# This migration was automatically generated on 2024.12.31. Edit as needed.
import psycopg

ID = 17
VERSION = "2024.12.31.2"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute('ALTER TABLE "bench_client" RENAME COLUMN "title" TO "name"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" RENAME COLUMN "title" TO "name"')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" RENAME COLUMN "title" TO "name"')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" RENAME COLUMN "title" TO "name"')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" RENAME COLUMN "title" TO "name"')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" RENAME COLUMN "title" TO "name"')


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
