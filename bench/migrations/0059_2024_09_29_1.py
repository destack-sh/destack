# This migration was automatically generated on 2024.09.29. Edit as needed.
import psycopg

ID = 59
VERSION = "2024.09.29.1"
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
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "combinator"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "combinator" smallint')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
