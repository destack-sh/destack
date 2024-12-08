# This migration was automatically generated on 2024.12.08. Edit as needed.
import psycopg

ID = 16
VERSION = "2024.12.08.0"
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
    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "selection" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
