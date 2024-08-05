# This migration was automatically generated on 2024.08.05. Edit as needed.
import psycopg

ID = 26
VERSION = "2024.08.05.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_membership
    await cur.execute(
        """
    CREATE TABLE "bench_membership" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "user_id" uuid NOT NULL,
        "is_owner" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_invite
    await cur.execute(
        """
    CREATE TABLE "bench_invite" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "user_id" uuid,
        "user_email" varchar,
        "is_owner" boolean NOT NULL DEFAULT false,
        "roles_id" uuid[],
        "roles_ck" uuid[],
        "roles_bench_id" uuid[]
    )
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_membership
    await cur.execute('DROP TABLE "bench_membership"')

    # bench_invite
    await cur.execute('DROP TABLE "bench_invite"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
