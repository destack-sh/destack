# This migration was automatically generated on 2025.05.10. Edit as needed.
import psycopg

ID = 24
VERSION = "2025.05.10.0"
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
    # bench_kit
    await cur.execute('DROP TABLE "bench_kit"')

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "kit_id"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "kit_id"
    """
    )

    # bench_service
    await cur.execute(
        """
    CREATE TABLE "bench_service" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
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
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "definition_id" uuid,
        "target_id" uuid,
        "target_ck" uuid,
        "target_type" smallint,
        "target_bench_id" uuid
    )
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "service_id" uuid
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "service_id" uuid
    """
    )

    # bench_service
    await cur.execute(
        'CREATE INDEX "bench_service_bench_idx_parent_id" ON "bench_service" USING BTREE (parent_id) INCLUDE (id)'
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
