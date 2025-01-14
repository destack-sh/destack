# This migration was automatically generated on 2025.01.14. Edit as needed.
import psycopg

ID = 30
VERSION = "2025.01.14.4"
HAS_GLOBAL = True
HAS_REGIONAL = False
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "roles_bench_id"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "roles_ck"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "roles_id"')


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
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "identity_id"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "roles_bench_id"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "roles_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "roles_id"')

    # bench_action
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "identity_bench_id"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "identity_ck"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "identity_id"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "roles_bench_id"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "roles_ck"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "roles_id"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
