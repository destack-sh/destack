# This migration was automatically generated on 2025.02.28. Edit as needed.
import psycopg

ID = 23
VERSION = "2025.02.28.2"
HAS_GLOBAL = False
HAS_REGIONAL = False
HAS_LOCAL = True


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
    pass


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_pipe
    await cur.execute('DROP TABLE "bench_pipe"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "pipe_bench_id"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "pipe_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "pipe_id"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "pipe_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "pipe_id"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "step"')

    # bench_link
    await cur.execute(
        """
    CREATE TABLE "bench_link" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid,
        "package_ck" uuid,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "computed_values" jsonb[],
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "source_id" uuid NOT NULL,
        "source_ck" uuid NOT NULL,
        "source_bench_id" uuid NOT NULL,
        "target_id" uuid NOT NULL,
        "target_ck" uuid NOT NULL,
        "target_bench_id" uuid NOT NULL,
        "run_options" jsonb,
        "trigger" smallint NOT NULL DEFAULT 1,
        "delay" interval,
        "color" jsonb
    )
    """
    )

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "link_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "link_ck" uuid')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "link_id" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "link_ck" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "link_bench_id" uuid')

    # bench_link
    await cur.execute(
        'CREATE INDEX "bench_link_bench_idx_parent_id" ON bench_link USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
