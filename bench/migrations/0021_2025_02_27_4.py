# This migration was automatically generated on 2025.02.27. Edit as needed.
import psycopg

ID = 21
VERSION = "2025.02.27.4"
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
    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "tool_selection" jsonb')

    # bench_collection
    await cur.execute(
        """
    CREATE TABLE "bench_collection" (
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
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "icon" jsonb,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "tags_ck" uuid[]
    )
    """
    )
    await cur.execute(
        'CREATE INDEX "bench_collection_bench_idx_parent_id" ON bench_collection USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
