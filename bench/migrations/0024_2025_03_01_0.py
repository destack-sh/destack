# This migration was automatically generated on 2025.03.01. Edit as needed.
import psycopg

ID = 24
VERSION = "2025.03.01.0"
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
    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "computed_values"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "computed_values"')

    # bench_choice
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "computed_values"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "computed_values"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "computed_values"')

    # bench_channel
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "computed_values"')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "computed_values"')

    # bench_database
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "computed_values"')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "computed_values"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "computed_values"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "page_bench_id"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "calls"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "client_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "machine_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "run_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "session_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "started_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "started_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "started_by_bench_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "started_by_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "terminated_by_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "terminated_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "terminated_by_bench_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "terminated_by_id"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "title"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "user_id"')

    # bench_option
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "computed_values"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "computed_values"')

    # bench_page
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "computed_values"')

    # bench_class
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "computed_values"')

    # bench_tag
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "computed_values"')

    # bench_implementation
    await cur.execute('ALTER TABLE "bench_implementation" DROP COLUMN "computed_values"')

    # bench_page
    await cur.execute('ALTER TABLE "bench_page" ADD COLUMN "parent_base_ck" uuid')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "ck" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "parent_ck" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "parent_type" smallint')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "package_id" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "package_ck" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "template_at" timestamp')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "name" varchar NOT NULL')
    await cur.execute(
        'ALTER TABLE "bench_plan" ADD COLUMN "order_key" varchar NOT NULL DEFAULT \'a0\'::character varying'
    )
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "icon" jsonb')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "block_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "block_ck" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "block_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "thread_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "tags_id" uuid[]')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "tags_ck" uuid[]')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "duration" interval')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "started_at" timestamp')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "terminated_at" timestamp')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "owned_by_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "owned_by_ck" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "owned_by_type" smallint')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "owned_by_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "owned_by_base_ck" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "owned_by_base_bench_id" uuid')

    # bench_task
    await cur.execute(
        """
    CREATE TABLE "bench_task" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "parent_base_ck" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "package_ck" uuid NOT NULL,
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
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "icon" jsonb,
        "text" jsonb,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "tags_ck" uuid[],
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "due_at" timestamp,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "error" jsonb,
        "owned_by_id" uuid,
        "owned_by_ck" uuid,
        "owned_by_type" smallint,
        "owned_by_bench_id" uuid,
        "owned_by_base_ck" uuid,
        "owned_by_base_bench_id" uuid,
        "node_id" uuid NOT NULL,
        "node_ck" uuid NOT NULL,
        "node_type" smallint NOT NULL,
        "node_bench_id" uuid NOT NULL,
        "clazz_id" uuid,
        "clazz_ck" uuid,
        "clazz_bench_id" uuid,
        "value_packed" jsonb
    )
    """
    )

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "parent_ck" uuid')
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "plan_ck" uuid')

    # bench_package
    await cur.execute(
        """
        ALTER TABLE bench_package    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE bench_dependency    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_page
    await cur.execute(
        """
        ALTER TABLE bench_page    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_choice
    await cur.execute(
        """
        ALTER TABLE bench_choice    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_class
    await cur.execute(
        """
        ALTER TABLE bench_class    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL,
        ALTER COLUMN "bench_type" SET DATA TYPE smallint
    """
    )

    # bench_option
    await cur.execute(
        """
        ALTER TABLE bench_option    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE bench_tag    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_flow
    await cur.execute(
        """
        ALTER TABLE bench_flow    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_action
    await cur.execute(
        """
        ALTER TABLE bench_action    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE bench_link    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_implementation
    await cur.execute(
        """
        ALTER TABLE bench_implementation    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_database
    await cur.execute(
        """
        ALTER TABLE bench_database    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_channel
    await cur.execute(
        """
        ALTER TABLE bench_channel    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE bench_role    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_task
    await cur.execute(
        'CREATE INDEX "bench_task_bench_idx_created_at" ON bench_task USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_task_bench_idx_parent_id" ON bench_task USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ALTER COLUMN "package_id" SET NOT NULL,
        ALTER COLUMN "package_ck" SET NOT NULL
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ALTER COLUMN "page_id" DROP NOT NULL,
        ALTER COLUMN "page_ck" DROP NOT NULL
    """
    )

    # bench_run_span
    await cur.execute(
        """
        ALTER TABLE bench_run_span    
        ALTER COLUMN "status" SET DEFAULT 2
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
