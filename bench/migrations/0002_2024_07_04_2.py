# This migration was automatically generated on 2024.07.04. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.07.04.2"
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
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "set_properties"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "set_properties"')

    # bench_link
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "set_properties"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "set_properties"')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "set_properties"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "set_properties"')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "set_properties"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "set_properties"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "set_properties"')

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "set_properties"')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "set_properties"')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "set_properties"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
