# This migration was automatically generated on 2025.04.13. Edit as needed.
import psycopg

ID = 56
VERSION = "2025.04.13.0"
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
    # bench_link
    await cur.execute('DROP TABLE "bench_link"')

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        DROP COLUMN "link_id"
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        DROP COLUMN "link_id"
    """
    )

    # bench_transition
    await cur.execute(
        """
    CREATE TABLE "bench_transition" (
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
        "mode" smallint NOT NULL DEFAULT 20,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "source_id" uuid NOT NULL,
        "source_bench_id" uuid NOT NULL,
        "target_id" uuid NOT NULL,
        "target_bench_id" uuid NOT NULL,
        "options" jsonb,
        "color" jsonb
    )
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "transition_id" uuid
    """
    )

    # bench_span
    await cur.execute(
        """
        ALTER TABLE "bench_span"    
        ADD COLUMN "transition_id" uuid
    """
    )

    # bench_transition
    await cur.execute(
        'CREATE INDEX "bench_transition_bench_idx_parent_id" ON "bench_transition" USING BTREE (parent_id) INCLUDE (id)'
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
