# This migration was automatically generated on 2024.11.29. Edit as needed.
import psycopg

ID = 8
VERSION = "2024.11.29.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_server
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_vault
    await cur.execute('ALTER TABLE "bench_vault" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_cache
    await cur.execute('ALTER TABLE "bench_cache" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_dependency
    await cur.execute(
        'ALTER TABLE "bench_dependency" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1'
    )

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" ADD COLUMN "mode" smallint NOT NULL DEFAULT 1')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
