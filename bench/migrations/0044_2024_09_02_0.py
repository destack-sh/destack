# This migration was automatically generated on 2024.09.02. Edit as needed.
import psycopg

ID = 44
VERSION = "2024.09.02.0"
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
    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "template_id"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "templated_epoch"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
