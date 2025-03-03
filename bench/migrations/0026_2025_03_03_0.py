# This migration was automatically generated on 2025.03.03. Edit as needed.
import psycopg

ID = 26
VERSION = "2025.03.03.0"
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
    # bench_link
    await cur.execute(
        'ALTER TABLE "bench_link" ADD COLUMN "is_manual" boolean NOT NULL DEFAULT false'
    )

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "name" varchar NOT NULL')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
