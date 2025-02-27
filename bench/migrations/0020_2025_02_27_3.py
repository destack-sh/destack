# This migration was automatically generated on 2025.02.27. Edit as needed.
import psycopg

ID = 20
VERSION = "2025.02.27.3"
HAS_GLOBAL = False
HAS_REGIONAL = True
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
    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "tags_ck" uuid[]')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "tags_ck" uuid[]')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "tags_ck" uuid[]')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "tags_ck" uuid[]')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "tags_ck" uuid[]')


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_action
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "machine_bench_id"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "machine_id"')

    # bench_page
    await cur.execute('ALTER TABLE "bench_page" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_page" ADD COLUMN "tags_ck" uuid[]')

    # bench_choice
    await cur.execute('ALTER TABLE "bench_choice" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_choice" ADD COLUMN "tags_ck" uuid[]')

    # bench_class
    await cur.execute('ALTER TABLE "bench_class" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_class" ADD COLUMN "tags_ck" uuid[]')

    # bench_tag
    await cur.execute(
        """
    CREATE TABLE "bench_tag" (
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
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "icon" jsonb,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "tags_ck" uuid[]
    )
    """
    )

    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_flow" ADD COLUMN "tags_ck" uuid[]')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "tags_ck" uuid[]')

    # bench_database
    await cur.execute('ALTER TABLE "bench_database" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_database" ADD COLUMN "tags_ck" uuid[]')

    # bench_channel
    await cur.execute('ALTER TABLE "bench_channel" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_channel" ADD COLUMN "tags_ck" uuid[]')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "tags_ck" uuid[]')

    # bench_tag
    await cur.execute(
        'CREATE INDEX "bench_tag_bench_idx_parent_id" ON bench_tag USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
