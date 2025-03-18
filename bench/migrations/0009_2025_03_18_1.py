# This migration was automatically generated on 2025.03.18. Edit as needed.
import psycopg

ID = 9
VERSION = "2025.03.18.1"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_team
    await cur.execute('DROP TABLE "bench_team"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_membership
    await cur.execute(
        """
        ALTER TABLE "bench_membership"    
        DROP COLUMN "to_bench_id",
        DROP COLUMN "to_id",
        DROP COLUMN "to_type"
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE "bench_space"    
        ADD COLUMN "channel_ck" uuid
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE "bench_channel"    
        ADD COLUMN "ck" uuid NOT NULL
    """
    )

    # bench_thread
    await cur.execute(
        """
        ALTER TABLE "bench_thread"    
        ADD COLUMN "channel_ck" uuid
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE "bench_message"    
        ADD COLUMN "channel_ck" uuid
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE "bench_notification"    
        ADD COLUMN "channel_ck" uuid
    """
    )

    # bench_team
    await cur.execute(
        """
    CREATE TABLE "bench_team" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "organization_id" uuid,
        "team_id" uuid,
        "team_ck" uuid,
        "team_bench_id" uuid,
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
        "mode" smallint NOT NULL DEFAULT 20,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "thread_ck" uuid,
        "tags_id" uuid[]
    )
    """
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE "bench_membership"    
        ADD COLUMN "member_ck" uuid NOT NULL,
        ADD COLUMN "member_type" smallint NOT NULL,
        ADD COLUMN "member_bench_id" uuid NOT NULL
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE "bench_run"    
        ADD COLUMN "channel_ck" uuid
    """
    )

    # bench_team
    await cur.execute(
        'CREATE INDEX "bench_team_bench_idx_parent_id" ON "bench_team" USING BTREE (parent_id) INCLUDE (id)'
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
