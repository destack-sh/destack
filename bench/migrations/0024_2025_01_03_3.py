# This migration was automatically generated on 2025.01.03. Edit as needed.
import psycopg

ID = 24
VERSION = "2025.01.03.3"
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
    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "computed_values" jsonb[]')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "computed_values" jsonb[]')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "computed_values" jsonb[]')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "computed_values" jsonb[]')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "computed_values" jsonb[]')

    # bench_action
    await cur.execute('ALTER TABLE "bench_action" ADD COLUMN "computed_values" jsonb[]')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "computed_values" jsonb[]')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
