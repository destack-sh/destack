# This migration was automatically generated on 2025.01.28. Edit as needed.
import psycopg

ID = 11
VERSION = "2025.01.28.1"
HAS_GLOBAL = True
HAS_REGIONAL = False
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "bench_id"')

    # bench_team
    await cur.execute(
        """
    CREATE TABLE "bench_team" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "organization_id" uuid,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "slug" varchar,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb
    )
    """
    )

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "parent_type" smallint')
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "organization_id" uuid')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" ADD COLUMN "parent_type" smallint')

    # bench_team
    await cur.execute(
        'ALTER TABLE "bench_team" ADD COLUMN "main_bench_id" uuid REFERENCES bench_bench ON DELETE SET NULL'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_team_bench_idx_slug" ON bench_team USING BTREE (slug)'
    )
    await cur.execute(
        'ALTER TABLE "bench_team" ADD CONSTRAINT "bench_team_bench_idx_slug" UNIQUE USING INDEX bench_team_bench_idx_slug'
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE bench_membership    
        ALTER COLUMN "bench_id" DROP NOT NULL
    """
    )

    # bench_invite
    await cur.execute(
        """
        ALTER TABLE bench_invite    
        ALTER COLUMN "bench_id" DROP NOT NULL
    """
    )


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
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
