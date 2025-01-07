# This migration was automatically generated on 2025.01.07. Edit as needed.
import psycopg

ID = 26
VERSION = "2025.01.07.0"
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
    await cur.execute(
        'ALTER TABLE "bench_action" RENAME COLUMN "delegate_bench_id" TO "tool_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_action" RENAME COLUMN "delegate_ck" TO "tool_ck"')
    await cur.execute('ALTER TABLE "bench_action" RENAME COLUMN "delegate_id" TO "tool_id"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
