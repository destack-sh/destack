# This migration was automatically generated on 2024.08.12. Edit as needed.
import psycopg

ID = 36
VERSION = "2024.08.12.0"
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
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "spans" jsonb[]')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "events" jsonb[]')
    await cur.execute(
        'UPDATE "bench_run" SET "spans" = ARRAY[]::jsonb[], "events" = ARRAY[]::jsonb[]'
    )
    await cur.execute('ALTER TABLE "bench_run" ALTER COLUMN "spans" DROP NOT NULL')
    await cur.execute('ALTER TABLE "bench_run" ALTER COLUMN "events" DROP NOT NULL')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
