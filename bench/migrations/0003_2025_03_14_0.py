# This migration was automatically generated on 2025.03.14. Edit as needed.
import psycopg

ID = 3
VERSION = "2025.03.14.0"
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
    # bench_page
    await cur.execute(
        """
        ALTER TABLE "bench_page"    
        DROP COLUMN "parent_base_id"
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE "bench_field"    
        DROP COLUMN "parent_ck"
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE "bench_action"    
        DROP COLUMN "parent_ck"
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE "bench_link"    
        DROP COLUMN "parent_ck"
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE "bench_trigger"    
        DROP COLUMN "parent_base_id",
        DROP COLUMN "parent_ck"
    """
    )

    # bench_run_span
    await cur.execute(
        """
        ALTER TABLE "bench_run_span"    
        DROP COLUMN "parent_base_id"
    """
    )

    # bench_interruption
    await cur.execute(
        """
        ALTER TABLE "bench_interruption"    
        DROP COLUMN "parent_base_id"
    """
    )

    # bench_log
    await cur.execute(
        """
        ALTER TABLE "bench_log"    
        DROP COLUMN "parent_base_id"
    """
    )

    # bench_plan
    await cur.execute(
        """
        ALTER TABLE "bench_plan"    
        DROP COLUMN "parent_base_id",
        DROP COLUMN "parent_ck"
    """
    )

    # bench_task
    await cur.execute(
        """
        ALTER TABLE "bench_task"    
        DROP COLUMN "parent_base_id",
        DROP COLUMN "parent_ck"
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "parent_base_id"
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        DROP COLUMN "parent_base_id",
        DROP COLUMN "parent_ck"
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
