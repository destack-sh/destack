# This migration was automatically generated on 2024.08.13. Edit as needed.
import psycopg

ID = 37
VERSION = "2024.08.13.0"
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
    # bench_log
    await cur.execute('ALTER TABLE "bench_log" ADD COLUMN "undo_of_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
