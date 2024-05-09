# This migration was automatically generated on 2024.05.09. Edit as needed.
import psycopg

ID = 6
VERSION = "2024.05.09.1"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_notice
    await cur.execute('ALTER TABLE "bench_notice" DROP COLUMN "parent_trigger_id"')

    # bench_message
    await cur.execute(
        """
    CREATE TABLE "bench_message" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL DEFAULT 0,
        "created_at" timestamp NOT NULL,
        "updated_at" timestamp NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "origin_id" uuid,
        "origin_ck" uuid,
        "origin_type" smallint,
        "origin_bench_id" uuid,
        "origin_base_ck" uuid,
        "origin_base_bench_id" uuid,
        "path" jsonb,
        "reply_to_id" uuid,
        "reply_to_ck" uuid,
        "reply_to_bench_id" uuid,
        "title" varchar,
        "text" jsonb,
        "value_packed" jsonb,
        "secret_value_packed" jsonb,
        "is_pinned" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_notice
    await cur.execute(
        'ALTER TABLE "bench_notice" DROP CONSTRAINT IF EXISTS "bench_notice_bench_check_one_parent", ADD CONSTRAINT "bench_notice_bench_check_one_parent" CHECK ((parent_block_id IS NOT NULL) OR (parent_field_id IS NOT NULL) OR (parent_step_id IS NOT NULL) OR (parent_view_id IS NOT NULL))'
    )

    # bench_message
    await cur.execute(
        'ALTER TABLE "bench_message" ADD COLUMN "parent_package_id" uuid REFERENCES bench_package ON DELETE CASCADE'
    )
    await cur.execute(
        'ALTER TABLE "bench_message" ADD COLUMN "parent_block_id" uuid REFERENCES bench_block ON DELETE CASCADE'
    )
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_package_deleted_at" ON bench_message USING BTREE (deleted_at, package_id)'
    )
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_package_archived_at" ON bench_message USING BTREE (archived_at, package_id)'
    )
    await cur.execute(
        'ALTER TABLE "bench_message" ADD CONSTRAINT "bench_message_bench_check_one_parent" CHECK ((parent_package_id IS NOT NULL) OR (parent_block_id IS NOT NULL))'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
