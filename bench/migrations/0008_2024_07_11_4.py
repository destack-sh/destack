# This migration was automatically generated on 2024.07.11. Edit as needed.
import psycopg

ID = 8
VERSION = "2024.07.11.4"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_blob
    await cur.execute('DROP TABLE "bench_blob"')

    # bench_file
    await cur.execute(
        """
    CREATE TABLE "bench_file" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
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
        "title" varchar NOT NULL,
        "size" bigint NOT NULL,
        "sha512" varchar NOT NULL,
        "mime_type" varchar NOT NULL,
        "retention" smallint NOT NULL,
        "expires_at" timestamp
    )
    """
    )

    # bench_secret
    await cur.execute(
        """
    CREATE TABLE "bench_secret" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
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
        "title" varchar NOT NULL,
        "value_type" jsonb NOT NULL,
        "value_packed" bytea
    )
    """
    )

    # bench_file
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_file_bench_idx_parent_id_sha512" ON bench_file USING BTREE (parent_id, sha512)'
    )
    await cur.execute(
        'ALTER TABLE "bench_file" ADD CONSTRAINT "bench_file_bench_idx_parent_id_sha512" UNIQUE USING INDEX bench_file_bench_idx_parent_id_sha512'
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
