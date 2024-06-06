# This migration was automatically generated on 2024.06.06. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.06.06.0"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_dependency" ADD COLUMN "templated_epoch" bigint')

    # bench_upgrade
    await cur.execute('ALTER TABLE "bench_upgrade" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_upgrade" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_upgrade" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_upgrade" ADD COLUMN "templated_epoch" bigint')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "templated_epoch" bigint')

    # bench_link
    await cur.execute('ALTER TABLE "bench_link" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_link" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_link" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_link" ADD COLUMN "templated_epoch" bigint')

    # bench_notice
    await cur.execute('ALTER TABLE "bench_notice" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_notice" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_notice" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_notice" ADD COLUMN "templated_epoch" bigint')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "templated_epoch" bigint')
    await cur.execute('ALTER TABLE "bench_block" ADD COLUMN "run" jsonb')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_trigger" ADD COLUMN "templated_epoch" bigint')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "template_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "template_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_field" ADD COLUMN "templated_epoch" bigint')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_query" ADD COLUMN "templated_epoch" bigint')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "templated_epoch" bigint')
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "is_template" boolean DEFAULT false')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "templated_epoch" bigint')
    await cur.execute('ALTER TABLE "bench_step" ADD COLUMN "run" jsonb')
    await cur.execute(
        'ALTER TABLE "bench_step" ADD COLUMN "is_template" boolean NOT NULL DEFAULT false'
    )

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_badge" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_badge" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_badge" ADD COLUMN "templated_epoch" bigint')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_role" ADD COLUMN "templated_epoch" bigint')

    # bench_identity
    await cur.execute('ALTER TABLE "bench_identity" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_identity" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_identity" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_identity" ADD COLUMN "templated_epoch" bigint')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_membership" ADD COLUMN "templated_epoch" bigint')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_invite" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_invite" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_invite" ADD COLUMN "templated_epoch" bigint')


async def downgrade_global(cur: psycopg.AsyncCursor):
    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "template_id"')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "template_id"')

    # bench_identity
    await cur.execute('ALTER TABLE "bench_identity" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_identity" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_identity" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_identity" DROP COLUMN "template_id"')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "template_id"')

    # bench_badge
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_badge" DROP COLUMN "template_id"')

    # bench_step
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "is_template"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "run"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_step" DROP COLUMN "template_id"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "is_template"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "template_id"')

    # bench_query
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_query" DROP COLUMN "template_id"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "template_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "template_base_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "template_id"')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "template_id"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "run"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "template_id"')

    # bench_notice
    await cur.execute('ALTER TABLE "bench_notice" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_notice" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_notice" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_notice" DROP COLUMN "template_id"')

    # bench_link
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "template_id"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "template_id"')

    # bench_upgrade
    await cur.execute('ALTER TABLE "bench_upgrade" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_upgrade" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_upgrade" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_upgrade" DROP COLUMN "template_id"')

    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "templated_epoch"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "template_bench_id"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "template_id"')


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "options" jsonb')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "attempts" jsonb[]')
    await cur.execute('UPDATE "bench_run" SET "attempts" = ARRAY[]::jsonb[]')
    await cur.execute('ALTER TABLE "bench_run" ALTER COLUMN "attempts" SET NOT NULL')


async def downgrade_local(cur: psycopg.AsyncCursor):
    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "attempts"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "options"')
