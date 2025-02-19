# This migration was automatically generated on 2025.02.19. Edit as needed.
import psycopg

ID = 9
VERSION = "2025.02.19.2"
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
    # bench_option
    await cur.execute(
        """
    CREATE TABLE "bench_option" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
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
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "icon" jsonb,
        "kind" smallint NOT NULL,
        "primitive_type" smallint,
        "bench_type" integer,
        "base_type_id" uuid,
        "base_type_ck" uuid,
        "base_type_type" smallint,
        "base_type_bench_id" uuid,
        "base_field_types" smallint[],
        "property_field_types" smallint[],
        "oneof_id" uuid,
        "oneof_ck" uuid,
        "oneof_type" smallint,
        "oneof_base_ck" uuid,
        "default_packed" jsonb,
        "format" smallint,
        "condition" jsonb,
        "constraint" jsonb,
        "is_required" boolean NOT NULL DEFAULT false,
        "is_list" boolean NOT NULL DEFAULT false,
        "is_secret" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "run_root_id" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "run_root_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "run_root_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "run_root_base_bench_id" uuid')

    # bench_option
    await cur.execute(
        'CREATE INDEX "bench_option_bench_idx_parent_id" ON bench_option USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
