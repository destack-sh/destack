# This migration was automatically generated on 2024.06.07. Edit as needed.
import psycopg

ID = 2
VERSION = "2024.06.07.0"
HAS_GLOBAL = True
HAS_LOCAL = False


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_notice
    await cur.execute('DROP TABLE "bench_notice"')

    # bench_issue
    await cur.execute(
        """
    CREATE TABLE "bench_issue" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "parent_base_ck" uuid,
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
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "kind" smallint NOT NULL,
        "type" smallint NOT NULL,
        "subject_id" uuid,
        "subject_ck" uuid,
        "subject_type" smallint,
        "subject_bench_id" uuid,
        "subject_base_ck" uuid,
        "subject_base_bench_id" uuid,
        "path" jsonb,
        "properties_ptr" jsonb[],
        "title" varchar,
        "text" jsonb
    )
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
