# This migration was automatically generated on 2025.02.04. Edit as needed.
import psycopg

ID = 18
VERSION = "2025.02.04.1"
HAS_GLOBAL = False
HAS_REGIONAL = True
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
    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "bitrate"')
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "channels"')
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "sample_rate"')


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "variables_packed"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "root_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "root_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "scope_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "scope_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "scope_type"')

    # bench_channel
    await cur.execute(
        """
    CREATE TABLE "bench_channel" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid,
        "package_ck" uuid,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "computed_values" jsonb[],
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 1,
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "icon" jsonb,
        "text" jsonb
    )
    """
    )

    # bench_thread
    await cur.execute(
        """
    CREATE TABLE "bench_thread" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 1,
        "channel_id" uuid,
        "channel_ck" uuid,
        "scope_id" uuid,
        "scope_ck" uuid,
        "scope_type" smallint,
        "run_id" uuid,
        "run_bench_id" uuid,
        "run_base_ck" uuid,
        "run_base_bench_id" uuid,
        "status" smallint NOT NULL DEFAULT 1,
        "closed_at" timestamp,
        "title" varchar,
        "text" jsonb
    )
    """
    )

    # bench_message
    await cur.execute(
        'ALTER TABLE "bench_message" ADD COLUMN "platform" smallint NOT NULL DEFAULT 1'
    )
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "channel_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "channel_base_ck" uuid')

    # bench_package
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_package_bench_idx_bench_id_slug" ON bench_package USING BTREE (bench_id, slug)'
    )

    # bench_channel
    await cur.execute(
        'CREATE INDEX "bench_channel_bench_idx_created_at" ON bench_channel USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_channel_bench_idx_parent_id" ON bench_channel USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_thread
    await cur.execute(
        'CREATE INDEX "bench_thread_bench_idx_created_at" ON bench_thread USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_thread_bench_idx_parent_id" ON bench_thread USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
