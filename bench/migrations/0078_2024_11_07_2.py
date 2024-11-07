# This migration was automatically generated on 2024.11.07. Edit as needed.
import psycopg

ID = 78
VERSION = "2024.11.07.2"
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
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "context" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
