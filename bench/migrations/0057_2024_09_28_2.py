# This migration was automatically generated on 2024.09.28. Edit as needed.
import psycopg

ID = 57
VERSION = "2024.09.28.2"
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
    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "combinator"')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "combinator" smallint')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
