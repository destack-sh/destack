# This migration was automatically generated on 2024.09.07. Edit as needed.
import psycopg

ID = 45
VERSION = "2024.09.07.0"
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
    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "run_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "run_base_bench_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
