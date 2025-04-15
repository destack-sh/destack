# This migration was automatically generated on 2025.04.15. Edit as needed.
import psycopg

ID = 60
VERSION = "2025.04.15.0"
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
    # bench_field
    await cur.execute(
        """
        ALTER TABLE "bench_field"    
        DROP COLUMN "oneof_ck",
        DROP COLUMN "oneof_id",
        DROP COLUMN "oneof_type"
    """
    )

    # bench_link
    await cur.execute(
        """
    CREATE TABLE "bench_link" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
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
        "archived_at" timestamp,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "claimed_by_id" uuid,
        "claimed_by_ck" uuid,
        "mode" smallint NOT NULL DEFAULT 20,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "definition_id" uuid,
        "region" smallint NOT NULL,
        "scaler_id" uuid,
        "scaler_ck" uuid,
        "scaler_bench_id" uuid,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "url" varchar,
        "expires_at" timestamp
    )
    """
    )

    # bench_claim
    await cur.execute(
        """
        ALTER TABLE "bench_claim"    
        ADD COLUMN "is_hidden" boolean NOT NULL DEFAULT false
    """
    )

    # bench_link
    await cur.execute(
        'CREATE INDEX "bench_link_bench_idx_parent_id" ON "bench_link" USING BTREE (parent_id) INCLUDE (id)'
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
