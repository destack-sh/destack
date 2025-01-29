# This migration was automatically generated on 2025.01.29. Edit as needed.
import psycopg

ID = 13
VERSION = "2025.01.29.1"
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
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "expires_at"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "is_pinned"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "name"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "origin_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "origin_bench_id"')

    # bench_trigger
    await cur.execute(
        """
    CREATE TABLE "bench_trigger" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "parent_base_ck" uuid,
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
        "text" jsonb,
        "effect" smallint NOT NULL,
        "interruption_id" uuid,
        "interruption_bench_id" uuid,
        "interruption_base_ck" uuid,
        "interruption_base_bench_id" uuid,
        "scope_id" uuid,
        "scope_ck" uuid,
        "scope_type" smallint,
        "scope_bench_id" uuid,
        "run_id" uuid,
        "run_bench_id" uuid,
        "run_base_ck" uuid,
        "run_base_bench_id" uuid,
        "status" smallint NOT NULL DEFAULT 10,
        "closed_at" timestamp
    )
    """
    )

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "parent_type" smallint')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "parent_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "failed_at" timestamp')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "sent_at" timestamp')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "received_at" timestamp')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "interruption_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "interruption_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "interruption_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "interruption_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "run_base_bench_id" uuid')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "title" varchar')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "text" jsonb')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "response" smallint')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "message_id" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "message_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "message_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "message_base_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "cancel_trigger_id" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "cancel_trigger_ck" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "cancel_trigger_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "complete_trigger_id" uuid')
    await cur.execute('ALTER TABLE "bench_interruption" ADD COLUMN "complete_trigger_ck" uuid')
    await cur.execute(
        'ALTER TABLE "bench_interruption" ADD COLUMN "complete_trigger_bench_id" uuid'
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE bench_package    
        ALTER COLUMN "package_id" DROP NOT NULL,
        ALTER COLUMN "package_ck" DROP NOT NULL
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE bench_dependency    
        ALTER COLUMN "package_id" DROP NOT NULL,
        ALTER COLUMN "package_ck" DROP NOT NULL
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ALTER COLUMN "package_id" DROP NOT NULL,
        ALTER COLUMN "package_ck" DROP NOT NULL
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN "package_id" DROP NOT NULL,
        ALTER COLUMN "package_ck" DROP NOT NULL
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN "package_id" DROP NOT NULL,
        ALTER COLUMN "package_ck" DROP NOT NULL
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE bench_action    
        ALTER COLUMN "package_id" DROP NOT NULL,
        ALTER COLUMN "package_ck" DROP NOT NULL
    """
    )

    # bench_pipe
    await cur.execute(
        """
        ALTER TABLE bench_pipe    
        ALTER COLUMN "package_id" DROP NOT NULL,
        ALTER COLUMN "package_ck" DROP NOT NULL
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ALTER COLUMN "package_id" DROP NOT NULL,
        ALTER COLUMN "package_ck" DROP NOT NULL
    """
    )

    # bench_message
    await cur.execute(
        """
        ALTER TABLE bench_message    
        ALTER COLUMN "status" SET DEFAULT 30
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
