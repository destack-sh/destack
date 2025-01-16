# This migration was automatically generated on 2025.01.16. Edit as needed.
import psycopg

ID = 32
VERSION = "2025.01.16.0"
HAS_GLOBAL = False
HAS_REGIONAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    pass


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "type" smallint NOT NULL')


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "is_hidden"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "is_name_shown"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
