# This migration was automatically generated on 2025.03.04. Edit as needed.
import psycopg

ID = 34
VERSION = "2025.03.04.1"
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
    # bench_action
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "inputs_packed"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "variables_packed"')

    # bench_run
    await cur.execute(
        'ALTER TABLE "bench_run" RENAME COLUMN "variables_packed" TO "resources_packed"'
    )

    # bench_field
    await cur.execute("UPDATE bench_field SET type = 40 WHERE type = 1")  # VARIABLE -> RESOURCE
    await cur.execute(
        "UPDATE bench_field SET type = 10 WHERE type = 2"
    )  # MEMBER -> MEMBER (new value)
    await cur.execute(
        "UPDATE bench_field SET type = 20 WHERE type = 3"
    )  # INPUT -> INPUT (new value)
    await cur.execute(
        "UPDATE bench_field SET type = 30 WHERE type = 4"
    )  # OUTPUT -> OUTPUT (new value)


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
