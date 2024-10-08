# This migration was automatically generated on 2024.10.08. Edit as needed.
import psycopg

ID = 65
VERSION = "2024.10.08.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "revision"')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "revision"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "revision"')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" DROP COLUMN "revision"')

    # bench_user
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "revision"')

    # bench_organization
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "revision"')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "revision"')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "revision"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "revision"')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "revision"')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "revision"')

    # bench_cache
    await cur.execute('ALTER TABLE "bench_cache" DROP COLUMN "revision"')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "revision"')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "revision"')

    # bench_vault
    await cur.execute('ALTER TABLE "bench_vault" DROP COLUMN "revision"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "revision"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "revision"')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "revision"')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "revision"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "revision"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "revision"')

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "revision"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "revision"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "revision"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "revision"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "revision"')

    # bench_branch
    await cur.execute('ALTER TABLE "bench_branch" DROP COLUMN "revision"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "revision"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "revision"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "revision"')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "revision"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "revision"')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "revision"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
