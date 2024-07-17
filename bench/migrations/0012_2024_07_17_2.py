# This migration was automatically generated on 2024.07.17. Edit as needed.
import psycopg

ID = 12
VERSION = "2024.07.17.2"
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
    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "dependency_scopes_bench_id"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "dependency_scopes_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "dependency_scopes_id"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
