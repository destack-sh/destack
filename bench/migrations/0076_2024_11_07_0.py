# This migration was automatically generated on 2024.11.07. Edit as needed.
import psycopg

ID = 76
VERSION = "2024.11.07.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_browser
    await cur.execute(
        """
    CREATE TABLE "bench_browser" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "current_status" smallint NOT NULL
    )
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
