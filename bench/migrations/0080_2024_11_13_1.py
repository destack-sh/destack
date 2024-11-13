# This migration was automatically generated on 2024.11.13. Edit as needed.
import psycopg

ID = 80
VERSION = "2024.11.13.1"
HAS_GLOBAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_view
    await cur.execute('TRUNCATE TABLE "bench_view"')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "constraint" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
