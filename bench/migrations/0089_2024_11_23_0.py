# This migration was automatically generated on 2024.11.23. Edit as needed.
import psycopg

ID = 89
VERSION = "2024.11.23.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "name"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "name"')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" ADD COLUMN "title" varchar')
    await cur.execute('UPDATE "bench_client" SET "title" = \'Client\'')
    await cur.execute('ALTER TABLE "bench_client" ALTER COLUMN "title" SET NOT NULL')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "type" smallint NOT NULL DEFAULT 1')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "title" varchar')
    await cur.execute('UPDATE "bench_machine" SET "title" = \'Machine\'')
    await cur.execute('ALTER TABLE "bench_machine" ALTER COLUMN "title" SET NOT NULL')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "killed_at" timestamp')

    # bench_stream
    await cur.execute(
        """
    CREATE TABLE "bench_stream" (
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
