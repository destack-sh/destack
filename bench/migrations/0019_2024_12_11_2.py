# This migration was automatically generated on 2024.12.12. Edit as needed.
import psycopg

ID = 19
VERSION = "2024.12.11.2"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "current_status"')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "current_status"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "bumped_at"')
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "current_status"')
    await cur.execute('ALTER TABLE "bench_server" DROP COLUMN "current_version"')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "current_status"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "current_version"')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" DROP COLUMN "current_status"')

    # bench_vault
    await cur.execute('ALTER TABLE "bench_vault" DROP COLUMN "current_status"')

    # bench_cache
    await cur.execute('ALTER TABLE "bench_cache" DROP COLUMN "current_status"')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "current_status"')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "current_status"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "current_cpu"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "current_ram"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "current_status"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "current_version"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "restarted_at"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "started_at"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "stopped_at"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "terminated_at"')

    # bench_server
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_server" ADD COLUMN "target_version" varchar')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "active_at" timestamp')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "target_version" varchar')

    # bench_drive
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_drive" ADD COLUMN "active_at" timestamp')

    # bench_vault
    await cur.execute('ALTER TABLE "bench_vault" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_vault" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_vault" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_vault" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_vault" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_vault" ADD COLUMN "active_at" timestamp')

    # bench_cache
    await cur.execute('ALTER TABLE "bench_cache" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_cache" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_cache" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_cache" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_cache" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_cache" ADD COLUMN "active_at" timestamp')

    # bench_machine
    await cur.execute(
        'ALTER TABLE "bench_machine" ADD COLUMN "occupancy" smallint NOT NULL DEFAULT 1'
    )
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "owned_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "owned_by_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "owned_by_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "owned_by_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute(
        'ALTER TABLE "bench_machine" ADD COLUMN "target_version" varchar DEFAULT \'2024.12.11.2\'::character varying'
    )
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "target_cpu" real')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "target_ram" real')
    await cur.execute('ALTER TABLE "bench_machine" ALTER COLUMN "occupancy" DROP DEFAULT')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "type" smallint NOT NULL DEFAULT 1')
    await cur.execute(
        'ALTER TABLE "bench_browser" ADD COLUMN "occupancy" smallint NOT NULL DEFAULT 1'
    )
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "owned_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "owned_by_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "owned_by_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "owned_by_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "active_at" timestamp')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "version" varchar')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "target_version" varchar')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "external_name" varchar')
    await cur.execute('ALTER TABLE "bench_browser" ALTER COLUMN "occupancy" DROP DEFAULT')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "occupancy" smallint NOT NULL DEFAULT 1')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "owned_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "owned_by_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "owned_by_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "owned_by_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "active_at" timestamp')
    await cur.execute('ALTER TABLE "bench_file" ALTER COLUMN "occupancy" DROP DEFAULT')

    # bench_stream
    await cur.execute(
        'ALTER TABLE "bench_stream" ADD COLUMN "occupancy" smallint NOT NULL DEFAULT 1'
    )
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "owned_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "owned_by_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "owned_by_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "owned_by_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "active_at" timestamp')
    await cur.execute('ALTER TABLE "bench_stream" ALTER COLUMN "occupancy" DROP DEFAULT')

    # bench_secret
    await cur.execute(
        'ALTER TABLE "bench_secret" ADD COLUMN "occupancy" smallint NOT NULL DEFAULT 1'
    )
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "owned_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "owned_by_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "owned_by_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "owned_by_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "activated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "deactivated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "reset_at" timestamp')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "suspended_at" timestamp')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "decommissioned_at" timestamp')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "active_at" timestamp')
    await cur.execute('ALTER TABLE "bench_secret" ALTER COLUMN "occupancy" DROP DEFAULT')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_pipe
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "condition_code"')
    await cur.execute('ALTER TABLE "bench_pipe" DROP COLUMN "mapping_code"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
