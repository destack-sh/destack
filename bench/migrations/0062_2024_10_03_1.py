# This migration was automatically generated on 2024.10.03. Edit as needed.
import psycopg

ID = 62
VERSION = "2024.10.03.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "archived_at"')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "archived_at"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "archived_at"')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" DROP COLUMN "archived_at"')

    # bench_user
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "archived_at"')

    # bench_organization
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "archived_at"')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "archived_at"')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "archived_at"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "archived_at"')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "archived_at"')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "archived_at"')

    # bench_cache
    await cur.execute('ALTER TABLE "bench_cache" DROP COLUMN "archived_at"')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "archived_at"')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "archived_at"')

    # bench_vault
    await cur.execute('ALTER TABLE "bench_vault" DROP COLUMN "archived_at"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_record_shared
    await cur.execute('DROP TABLE "bench_record_shared"')

    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "archived_at"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "archived_at"')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "archived_at"')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "archived_at"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "archived_at"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "archived_at"')

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "archived_at"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "archived_at"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "archived_at"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "archived_at"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "archived_at"')

    # bench_branch
    await cur.execute('ALTER TABLE "bench_branch" DROP COLUMN "archived_at"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "archived_at"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "archived_at"')

    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "archived_at"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "archived_at"')

    # bench_signal
    await cur.execute('ALTER TABLE "bench_signal" DROP COLUMN "archived_at"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "archived_at"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
