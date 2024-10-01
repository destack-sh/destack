# This migration was automatically generated on 2024.10.01. Edit as needed.
import psycopg

ID = 60
VERSION = "2024.10.01.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute("ALTER TABLE bench_file DROP COLUMN duration")
    await cur.execute("ALTER TABLE bench_file ADD COLUMN duration interval")


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_session
    await cur.execute("TRUNCATE TABLE bench_session")
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "duration"')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "duration" interval')

    # bench_run
    await cur.execute("TRUNCATE TABLE bench_run")
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "duration"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "cached_duration"')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "cached_duration" interval')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "duration" interval')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
