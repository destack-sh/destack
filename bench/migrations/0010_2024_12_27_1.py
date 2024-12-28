# This migration was automatically generated on 2024.12.28. Edit as needed.
import psycopg

ID = 10
VERSION = "2024.12.27.1"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "server_bench_id"')
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "server_id"')

    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "main_cache_id"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "main_drive_id"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "main_server_id"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "main_vault_id"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_drive
    await cur.execute('DROP TABLE "bench_drive"')

    # bench_server
    await cur.execute('DROP TABLE "bench_server"')

    # bench_vault
    await cur.execute('DROP TABLE "bench_vault"')

    # bench_cache
    await cur.execute('DROP TABLE "bench_cache"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "parent_type"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "target_cpu"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "target_ram"')

    # bench_scaler
    await cur.execute(
        """
    CREATE TABLE "bench_scaler" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "strategy" smallint NOT NULL,
        "target_count" integer NOT NULL DEFAULT 0,
        "min_count" integer NOT NULL DEFAULT 0,
        "max_count" integer NOT NULL DEFAULT 16,
        "min_ready_count" integer NOT NULL DEFAULT 0,
        "is_active" boolean NOT NULL DEFAULT true,
        "mode" smallint NOT NULL DEFAULT 2
    )
    """
    )

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "type" smallint NOT NULL DEFAULT 1')

    # bench_machine
    await cur.execute(
        """
        ALTER TABLE bench_machine    
        ALTER COLUMN "type" SET DEFAULT 1,
        ALTER COLUMN "cpu" SET DEFAULT 1,
        ALTER COLUMN "ram" SET DEFAULT 1
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "server_id"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "server_id"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "server_id"')

    # bench_action
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "delegate_type"')

    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" DROP COLUMN "server_id"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
