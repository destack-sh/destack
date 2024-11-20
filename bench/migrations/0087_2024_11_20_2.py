# This migration was automatically generated on 2024.11.20. Edit as needed.
import psycopg

ID = 87
VERSION = "2024.11.20.2"
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
    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "text" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
