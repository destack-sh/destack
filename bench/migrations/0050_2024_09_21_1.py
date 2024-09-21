# This migration was automatically generated on 2024.09.21. Edit as needed.
import psycopg

ID = 50
VERSION = "2024.09.21.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_vault
    await cur.execute(
        """
    CREATE TABLE "bench_vault" (
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
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "current_status" smallint NOT NULL
    )
    """
    )

    # bench_cache
    await cur.execute(
        """
    CREATE TABLE "bench_cache" (
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
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "current_status" smallint NOT NULL
    )
    """
    )

    # bench_file
    await cur.execute(
        """
    CREATE TABLE "bench_file" (
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
        "retention" smallint NOT NULL,
        "expires_at" timestamp,
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "current_status" smallint NOT NULL,
        "kind" smallint NOT NULL,
        "title" varchar NOT NULL,
        "external_url" varchar,
        "inline_content" bytea,
        "coarse_type" smallint NOT NULL,
        "mime_type" varchar,
        "format" integer,
        "size" bigint NOT NULL,
        "sha256" varchar,
        "width" integer,
        "height" integer,
        "aspect_ratio" real,
        "codec" varchar,
        "duration" real,
        "bitrate" integer,
        "channels" integer,
        "sample_rate" integer
    )
    """
    )

    # bench_secret
    await cur.execute(
        """
    CREATE TABLE "bench_secret" (
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
        "name" varchar NOT NULL,
        "title" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "current_status" smallint NOT NULL,
        "value_type" jsonb NOT NULL,
        "value_packed" bytea
    )
    """
    )

    # bench_bench
    await cur.execute(
        'ALTER TABLE "bench_bench" ADD COLUMN "main_vault_id" uuid REFERENCES bench_vault ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_bench" ADD COLUMN "main_cache_id" uuid REFERENCES bench_cache ON DELETE SET NULL'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_secret
    await cur.execute('DROP TABLE "bench_secret"')

    # bench_file
    await cur.execute('DROP TABLE "bench_file"')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
