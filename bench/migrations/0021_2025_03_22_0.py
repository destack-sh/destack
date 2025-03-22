# This migration was automatically generated on 2025.03.22. Edit as needed.
import psycopg

ID = 21
VERSION = "2025.03.22.0"
HAS_GLOBAL = False
HAS_REGIONAL = True
HAS_LOCAL = False


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
    # bench_browser
    await cur.execute('DROP TABLE "bench_browser"')

    # bench_application
    await cur.execute(
        """
    CREATE TABLE "bench_application" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 20,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "thread_ck" uuid,
        "tags_id" uuid[],
        "region" smallint NOT NULL,
        "scaler_id" uuid,
        "scaler_bench_id" uuid,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "external_name" varchar,
        "external_id" varchar
    )
    """
    )
    await cur.execute(
        'CREATE INDEX "bench_application_bench_idx_parent_id" ON "bench_application" USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
