# This migration was automatically generated on 2024.12.05. Edit as needed.
import psycopg

ID = 13
VERSION = "2024.12.05.1"
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
    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "partial_scope" smallint')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
