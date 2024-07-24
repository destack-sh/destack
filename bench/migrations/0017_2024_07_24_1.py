# This migration was automatically generated on 2024.07.24. Edit as needed.
import psycopg

ID = 17
VERSION = "2024.07.24.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_file
    await cur.execute('DROP TABLE "bench_file"')

    # bench_secret
    await cur.execute('DROP TABLE "bench_secret"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "template_id"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "templated_epoch"')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "template_id"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "templated_epoch"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "ck"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "ck"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "parent_ck" uuid')
    await cur.execute('UPDATE "bench_space" SET parent_ck = parent_id')
    await cur.execute('ALTER TABLE "bench_space" ALTER COLUMN "parent_ck" SET NOT NULL')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "parent_type" smallint')
    await cur.execute('UPDATE "bench_space" SET parent_type = 1001')
    await cur.execute('ALTER TABLE "bench_space" ALTER COLUMN "parent_type" SET NOT NULL')

    # bench_secret
    await cur.execute(
        """
    CREATE TABLE "bench_secret" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
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
        """
    CREATE TABLE "bench_file" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
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
        "retention" smallint,
        "expires_at" timestamp,
        "kind" smallint NOT NULL,
        "drive_id" uuid,
        "url" varchar,
        "title" varchar NOT NULL,
        "content" bytea,
        "coarse_type" smallint NOT NULL,
        "mime_type" varchar NOT NULL,
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
    await cur.execute(
        'CREATE INDEX "bench_file_bench_idx_drive_id_sha256" ON bench_file USING BTREE (drive_id, sha256)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
