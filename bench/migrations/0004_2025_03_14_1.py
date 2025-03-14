# This migration was automatically generated on 2025.03.14. Edit as needed.
import psycopg

ID = 4
VERSION = "2025.03.14.1"
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
    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "resources_packed"
    """
    )

    # bench_claim
    await cur.execute(
        """
    CREATE TABLE "bench_claim" (
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
        "status" smallint NOT NULL DEFAULT 10,
        "duration" interval,
        "opened_at" timestamp,
        "granted_at" timestamp,
        "closed_at" timestamp,
        "resource_id" uuid,
        "resource_type" smallint,
        "resource_bench_id" uuid,
        "resource_selection" jsonb,
        "session_id" uuid,
        "client_id" uuid,
        "computer_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_browser
    await cur.execute(
        """
        ALTER TABLE "bench_browser"    
        ALTER COLUMN "type" DROP DEFAULT
    """
    )

    # bench_claim
    await cur.execute(
        'CREATE INDEX "bench_claim_bench_idx_parent_id" ON "bench_claim" USING BTREE (parent_id) INCLUDE (id)'
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
