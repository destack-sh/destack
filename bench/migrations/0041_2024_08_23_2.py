# This migration was automatically generated on 2024.08.23. Edit as needed.
import psycopg

ID = 41
VERSION = "2024.08.23.2"
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
    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "pipes"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "policies"')

    # bench_pipe
    await cur.execute(
        """
    CREATE TABLE "bench_pipe" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "source_id" uuid NOT NULL,
        "source_ck" uuid NOT NULL,
        "source_bench_id" uuid NOT NULL,
        "source_port" jsonb NOT NULL,
        "target_id" uuid NOT NULL,
        "target_ck" uuid NOT NULL,
        "target_bench_id" uuid NOT NULL,
        "target_port" jsonb NOT NULL,
        "filter_type" smallint,
        "line" jsonb,
        "color" jsonb
    )
    """
    )

    # bench_port
    await cur.execute(
        """
    CREATE TABLE "bench_port" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "name" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "type" smallint NOT NULL,
        "side" smallint NOT NULL,
        "field_id" uuid,
        "field_ck" uuid,
        "field_bench_id" uuid,
        "field_base_ck" uuid,
        "field_base_bench_id" uuid,
        "value_type" jsonb,
        "value_packed" jsonb
    )
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
