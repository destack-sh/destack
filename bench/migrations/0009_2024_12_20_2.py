# This migration was automatically generated on 2024.12.20. Edit as needed.
import psycopg

ID = 9
VERSION = "2024.12.20.2"
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
    # bench_step
    await cur.execute('DROP TABLE "bench_step"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "step_bench_id"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "step_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "step_id"')

    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" DROP COLUMN "step_bench_id"')
    await cur.execute('ALTER TABLE "bench_interrupt" DROP COLUMN "step_ck"')
    await cur.execute('ALTER TABLE "bench_interrupt" DROP COLUMN "step_id"')

    # bench_action
    await cur.execute(
        """
    CREATE TABLE "bench_action" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "package_id" uuid NOT NULL,
        "package_ck" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
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
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "icon" jsonb,
        "text" jsonb,
        "run_options" jsonb,
        "roles_id" uuid[],
        "roles_ck" uuid[],
        "roles_bench_id" uuid[],
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid,
        "code" jsonb,
        "delegate_id" uuid,
        "delegate_ck" uuid,
        "delegate_type" smallint,
        "delegate_bench_id" uuid,
        "position" jsonb,
        "mode" smallint NOT NULL DEFAULT 2
    )
    """
    )

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "action_id" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "action_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "action_bench_id" uuid')

    # bench_interrupt
    await cur.execute('ALTER TABLE "bench_interrupt" ADD COLUMN "action_id" uuid')
    await cur.execute('ALTER TABLE "bench_interrupt" ADD COLUMN "action_ck" uuid')
    await cur.execute('ALTER TABLE "bench_interrupt" ADD COLUMN "action_bench_id" uuid')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
