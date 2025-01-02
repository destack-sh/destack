# This migration was automatically generated on 2025.01.02. Edit as needed.
import psycopg

ID = 20
VERSION = "2025.01.02.2"
HAS_GLOBAL = False
HAS_REGIONAL = False
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
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_action
    await cur.execute('ALTER TABLE "bench_action" ADD COLUMN "calls" jsonb[] DEFAULT \'{}\'')
    await cur.execute('ALTER TABLE "bench_action" ALTER COLUMN "calls" DROP DEFAULT')
    await cur.execute('ALTER TABLE "bench_action" ALTER COLUMN "calls" SET NOT NULL')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "name" varchar')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
