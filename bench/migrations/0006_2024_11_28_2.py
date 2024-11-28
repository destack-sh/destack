# This migration was automatically generated on 2024.11.28. Edit as needed.
import psycopg

ID = 6
VERSION = "2024.11.28.2"
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
    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" ADD COLUMN "inputs_packed" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
