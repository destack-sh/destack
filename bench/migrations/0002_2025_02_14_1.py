# This migration was automatically generated on 2025.02.14. Edit as needed.
import psycopg

ID = 2
VERSION = "2025.02.14.1"
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
    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "identity_id" uuid')
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "identity_ck" uuid')
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "identity_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "roles_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "roles_ck" uuid[]')
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "roles_bench_id" uuid[]')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "color" smallint DEFAULT 41')

    # bench_identity
    await cur.execute('ALTER TABLE "bench_identity" ADD COLUMN "color" smallint DEFAULT 41')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
