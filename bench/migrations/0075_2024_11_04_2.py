# This migration was automatically generated on 2024.11.04. Edit as needed.
import psycopg

ID = 75
VERSION = "2024.11.04.2"
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
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "filter"')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "condition_code" jsonb')
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "mapping_code" jsonb')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
