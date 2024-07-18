# This migration was automatically generated on 2024.07.18. Edit as needed.
import psycopg

ID = 13
VERSION = "2024.07.18.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "content" bytea')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "url" varchar')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
