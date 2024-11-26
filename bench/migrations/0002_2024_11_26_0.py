# This migration was automatically generated on 2024.11.26. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.11.26.0"
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
    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "client_bench_id"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "machine_bench_id"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "server_bench_id"')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "session_id" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "run_root_id" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "run_root_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "identity_id" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "identity_ck" uuid')
    await cur.execute('ALTER TABLE "bench_session" ADD COLUMN "identity_bench_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
