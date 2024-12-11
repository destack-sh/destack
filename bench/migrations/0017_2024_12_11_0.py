# This migration was automatically generated on 2024.12.11. Edit as needed.
import psycopg

ID = 17
VERSION = "2024.12.11.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "title" varchar NOT NULL')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "title" varchar NOT NULL')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
