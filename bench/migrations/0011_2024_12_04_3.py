# This migration was automatically generated on 2024.12.04. Edit as needed.
import psycopg

ID = 11
VERSION = "2024.12.04.3"
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
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "dependency_bench_id"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "parent_type"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "scopes_bench_id"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "scopes_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "scopes_id"')
    await cur.execute(
        'ALTER TABLE "bench_dependency" ADD COLUMN "dependency_version_id" uuid NOT NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_dependency" ADD COLUMN "dependency_version_bench_id" uuid NOT NULL'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
