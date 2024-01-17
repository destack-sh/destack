# This migration was automatically generated on 2024.01.17. Edit as needed.
import psycopg

ID = 3
VERSION = "2024.01.17.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_module
    await cur.execute("DROP INDEX bench_projectversion_project_id_a8fd4b48")

    # bench_field
    await cur.execute("DROP INDEX bench_field_module_id_fd7c14c9")
    await cur.execute("DROP INDEX bench_simpletypenode_statement_id_0c6968df")
    await cur.execute("DROP INDEX bench_field_bench_idx_module_id")

    # bench_bench
    await cur.execute("DROP INDEX bench_project_organization_id_beed2131")
    await cur.execute("DROP INDEX bench_project_user_id_1cbf531a")

    # bench_statement
    await cur.execute("DROP INDEX bench_statement_parent_id_0708bb5b")
    await cur.execute("DROP INDEX bench_statement_project_version_id_7cb81a39")
    await cur.execute("DROP INDEX bench_statement_bench_idx_module_id")

    # bench_tagging
    await cur.execute("DROP INDEX bench_tagging_module_id_64a4aaba")
    await cur.execute("DROP INDEX bench_tagging_bench_idx_module_id")
    await cur.execute("DROP INDEX bench_tagging_statement_id_d2aa61d0")

    # bench_trigger
    await cur.execute("DROP INDEX bench_trigger_module_id_cfb4ac09")
    await cur.execute("DROP INDEX bench_trigger_statement_id_ec859879")
    await cur.execute("DROP INDEX bench_trigger_bench_idx_module_id")

    # bench_link
    await cur.execute("DROP INDEX bench_link_bench_idx_module_id")

    # bench_issue
    await cur.execute("DROP INDEX bench_issue_bench_idx_module_id")
    await cur.execute("DROP INDEX bench_issue_project_version_id_761a0549")
    await cur.execute("DROP INDEX bench_issue_statement_id_29579c23")
    await cur.execute("DROP INDEX bench_issue_file_id_87bab795")

    # bench_file
    await cur.execute("DROP INDEX bench_file_parent_id_c49f463c")
    await cur.execute("DROP INDEX bench_file_project_version_id_3b7023e0")
    await cur.execute("DROP INDEX bench_file_bench_idx_module_id")

    # bench_view
    await cur.execute("DROP INDEX bench_view_bench_idx_module_id")

    # bench_bench
    await cur.execute(
        "CREATE UNIQUE INDEX bench_bench_bench_idx_slug ON bench_bench USING BTREE (slug)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_bench    
        DROP CONSTRAINT IF EXISTS bench_bench_bench_unique_slug, ADD CONSTRAINT bench_bench_bench_unique_slug UNIQUE USING INDEX bench_bench_bench_idx_slug
    """
    )

    # bench_badge
    await cur.execute(
        "CREATE UNIQUE INDEX bench_badge_bench_idx_link_token ON bench_badge USING BTREE (link_token)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_badge    
        DROP CONSTRAINT IF EXISTS bench_badge_bench_unique_link_token, ADD CONSTRAINT bench_badge_bench_unique_link_token UNIQUE USING INDEX bench_badge_bench_idx_link_token
    """
    )

    # bench_handle
    await cur.execute(
        "CREATE UNIQUE INDEX bench_handle_bench_idx_slug ON bench_handle USING BTREE (slug)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_handle    
        DROP CONSTRAINT IF EXISTS bench_handle_bench_unique_slug, ADD CONSTRAINT bench_handle_bench_unique_slug UNIQUE USING INDEX bench_handle_bench_idx_slug
    """
    )

    # bench_user
    await cur.execute(
        "CREATE UNIQUE INDEX bench_user_bench_idx_slug ON bench_user USING BTREE (slug)"
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_user_bench_idx_email ON bench_user USING BTREE (email)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_user    
        DROP CONSTRAINT IF EXISTS bench_user_bench_unique_slug, ADD CONSTRAINT bench_user_bench_unique_slug UNIQUE USING INDEX bench_user_bench_idx_slug,
        DROP CONSTRAINT IF EXISTS bench_user_bench_unique_email, ADD CONSTRAINT bench_user_bench_unique_email UNIQUE USING INDEX bench_user_bench_idx_email
    """
    )

    # bench_organization
    await cur.execute(
        "CREATE UNIQUE INDEX bench_organization_bench_idx_slug ON bench_organization USING BTREE (slug)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_organization    
        DROP CONSTRAINT IF EXISTS bench_organization_bench_unique_slug, ADD CONSTRAINT bench_organization_bench_unique_slug UNIQUE USING INDEX bench_organization_bench_idx_slug
    """
    )

    # bench_client
    await cur.execute(
        "CREATE UNIQUE INDEX bench_client_bench_idx_access_token ON bench_client USING BTREE (access_token)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_client    
        DROP CONSTRAINT IF EXISTS bench_client_bench_unique_access_token, ADD CONSTRAINT bench_client_bench_unique_access_token UNIQUE USING INDEX bench_client_bench_idx_access_token
    """
    )

    # bench_worker
    await cur.execute(
        "CREATE UNIQUE INDEX bench_worker_bench_idx_external_id ON bench_worker USING BTREE (external_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_worker    
        DROP CONSTRAINT IF EXISTS bench_worker_bench_unique_external_id, ADD CONSTRAINT bench_worker_bench_unique_external_id UNIQUE USING INDEX bench_worker_bench_idx_external_id
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError()


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    pass


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError()
