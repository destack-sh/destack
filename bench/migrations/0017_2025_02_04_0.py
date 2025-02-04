# This migration was automatically generated on 2025.02.04. Edit as needed.
import psycopg

ID = 17
VERSION = "2025.02.04.0"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_handle
    await cur.execute(
        'CREATE INDEX "bench_handle_bench_idx_parent_id" ON bench_handle USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_team
    await cur.execute(
        'CREATE INDEX "bench_team_bench_idx_parent_id" ON bench_team USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_membership
    await cur.execute(
        'CREATE INDEX "bench_membership_bench_idx_parent_id" ON bench_membership USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_invite
    await cur.execute(
        'CREATE INDEX "bench_invite_bench_idx_parent_id" ON bench_invite USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_client
    await cur.execute(
        'CREATE INDEX "bench_client_bench_idx_parent_id" ON bench_client USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_scaler
    await cur.execute(
        'CREATE INDEX "bench_scaler_bench_idx_parent_id" ON bench_scaler USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_store
    await cur.execute(
        'CREATE INDEX "bench_store_bench_idx_parent_id" ON bench_store USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_machine
    await cur.execute(
        'CREATE INDEX "bench_machine_bench_idx_parent_id" ON bench_machine USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_browser
    await cur.execute(
        'CREATE INDEX "bench_browser_bench_idx_parent_id" ON bench_browser USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_file
    await cur.execute(
        'CREATE INDEX "bench_file_bench_idx_parent_id" ON bench_file USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_stream
    await cur.execute(
        'CREATE INDEX "bench_stream_bench_idx_parent_id" ON bench_stream USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_secret
    await cur.execute(
        'CREATE INDEX "bench_secret_bench_idx_parent_id" ON bench_secret USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "origin_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "origin_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "origin_type"')

    # bench_package
    await cur.execute(
        'ALTER TABLE "bench_package" DROP CONSTRAINT "bench_package_bench_idx_bench_id_slug"'
    )

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "scope_id" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "scope_ck" uuid')
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "scope_type" smallint')

    # bench_package
    await cur.execute(
        'CREATE INDEX "bench_package_bench_idx_parent_id" ON bench_package USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_dependency
    await cur.execute(
        'CREATE INDEX "bench_dependency_bench_idx_parent_id" ON bench_dependency USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_block
    await cur.execute(
        'CREATE INDEX "bench_block_bench_idx_parent_id" ON bench_block USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_field
    await cur.execute(
        'CREATE INDEX "bench_field_bench_idx_parent_id" ON bench_field USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_view
    await cur.execute(
        'CREATE INDEX "bench_view_bench_idx_parent_id" ON bench_view USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_action
    await cur.execute(
        'CREATE INDEX "bench_action_bench_idx_parent_id" ON bench_action USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_pipe
    await cur.execute(
        'CREATE INDEX "bench_pipe_bench_idx_parent_id" ON bench_pipe USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ALTER COLUMN "scope_id" SET NOT NULL,
        ALTER COLUMN "scope_ck" SET NOT NULL,
        ALTER COLUMN "scope_type" SET NOT NULL,
        ALTER COLUMN "scope_bench_id" SET NOT NULL
    """
    )
    await cur.execute(
        'CREATE INDEX "bench_trigger_bench_idx_parent_id" ON bench_trigger USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_space
    await cur.execute(
        'CREATE INDEX "bench_space_bench_idx_parent_id" ON bench_space USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_message
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_parent_id" ON bench_message USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_session
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_parent_id" ON bench_session USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_run
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_parent_id" ON bench_run USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_run_span
    await cur.execute(
        'CREATE INDEX "bench_run_span_bench_idx_parent_id" ON bench_run_span USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_run_plan
    await cur.execute(
        'CREATE INDEX "bench_run_plan_bench_idx_parent_id" ON bench_run_plan USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_interruption
    await cur.execute(
        'CREATE INDEX "bench_interruption_bench_idx_parent_id" ON bench_interruption USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_log
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_parent_id" ON bench_log USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
