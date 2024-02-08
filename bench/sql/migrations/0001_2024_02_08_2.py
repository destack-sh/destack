# This migration was automatically generated on 2024.02.08. Edit as needed.
import psycopg

ID = 1
VERSION = "2024.02.08.2"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_migration
    await cur.execute(
        """
    CREATE TABLE bench_migration (
        id integer NOT NULL PRIMARY KEY,
        version varchar NOT NULL,
        has_global boolean NOT NULL,
        has_local boolean NOT NULL,
        applied_at timestamp
    )
    """
    )

    # bench_query
    await cur.execute(
        """
    CREATE TABLE bench_query (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        name varchar,
        order_key varchar,
        node_type varchar NOT NULL,
        bases_block_ck uuid[],
        filter jsonb,
        sort jsonb[]
    )
    """
    )

    # bench_server
    await cur.execute(
        """
    CREATE TABLE bench_server (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        profile varchar NOT NULL,
        image jsonb,
        version varchar,
        sleep boolean NOT NULL DEFAULT true,
        status varchar NOT NULL,
        current_profile varchar,
        current_image jsonb,
        current_version varchar,
        last_active_at timestamp,
        last_bumped_at timestamp,
        external_id varchar,
        access_token bytea
    )
    """
    )

    # bench_user
    await cur.execute(
        """
    CREATE TABLE bench_user (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        slug varchar,
        name varchar,
        text jsonb,
        email varchar NOT NULL,
        password_salt bytea,
        password_hash bytea,
        last_logged_in_at timestamp,
        is_staff boolean NOT NULL DEFAULT false,
        is_activated boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_cache
    await cur.execute(
        """
    CREATE TABLE bench_cache (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL
    )
    """
    )

    # bench_identity
    await cur.execute(
        """
    CREATE TABLE bench_identity (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        type_block_ck uuid NOT NULL
    )
    """
    )

    # bench_role
    await cur.execute(
        """
    CREATE TABLE bench_role (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        type_block_ck uuid NOT NULL
    )
    """
    )

    # bench_skip
    await cur.execute(
        """
    CREATE TABLE bench_skip (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        reference_block_ck uuid NOT NULL,
        reference_trigger_ck uuid NOT NULL,
        reference_tag_ck uuid NOT NULL,
        reference_field_ck uuid NOT NULL,
        reference_record_ck uuid NOT NULL,
        reference_query_ck uuid NOT NULL,
        reference_view_ck uuid NOT NULL,
        reference_notice_ck uuid NOT NULL,
        reference_skip_ck uuid NOT NULL,
        reference_space_ck uuid NOT NULL,
        order_key varchar
    )
    """
    )

    # bench_badge
    await cur.execute(
        """
    CREATE TABLE bench_badge (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        type varchar NOT NULL,
        name varchar,
        delegated_policies jsonb[] NOT NULL,
        expires_at timestamp,
        link_token uuid,
        link_password bytea,
        link_password_hash bytea,
        key_value bytea,
        key_value_hash bytea
    )
    """
    )

    # bench_link
    await cur.execute(
        """
    CREATE TABLE bench_link (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        reference_block_ck uuid NOT NULL,
        reference_trigger_ck uuid NOT NULL,
        reference_tag_ck uuid NOT NULL,
        reference_field_ck uuid NOT NULL,
        reference_record_ck uuid NOT NULL,
        reference_query_ck uuid NOT NULL,
        reference_view_ck uuid NOT NULL,
        reference_notice_ck uuid NOT NULL,
        reference_skip_ck uuid NOT NULL,
        reference_space_ck uuid NOT NULL,
        order_key varchar
    )
    """
    )

    # bench_drive
    await cur.execute(
        """
    CREATE TABLE bench_drive (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        handle varchar
    )
    """
    )

    # bench_block
    await cur.execute(
        """
    CREATE TABLE bench_block (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        type varchar NOT NULL DEFAULT 2,
        policies jsonb[],
        bases_block_ck uuid[],
        builtin_base jsonb,
        is_page boolean NOT NULL DEFAULT false,
        is_module boolean NOT NULL DEFAULT false,
        is_unique_name boolean NOT NULL DEFAULT false,
        name varchar,
        order_key varchar,
        dynamic_key varchar,
        text jsonb,
        value_packed jsonb,
        secret_value_packed bytea,
        code varchar,
        icon jsonb,
        reference_block_ck uuid,
        delegated_policies jsonb[]
    )
    """
    )

    # bench_view
    await cur.execute(
        """
    CREATE TABLE bench_view (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        type varchar NOT NULL,
        name varchar,
        icon jsonb
    )
    """
    )

    # bench_package
    await cur.execute(
        """
    CREATE TABLE bench_package (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        slug varchar,
        text jsonb,
        policies jsonb[],
        is_paused boolean NOT NULL DEFAULT false,
        is_partial boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_tag
    await cur.execute(
        """
    CREATE TABLE bench_tag (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        value_packed jsonb,
        reference_block_ck uuid
    )
    """
    )

    # bench_notification
    await cur.execute(
        """
    CREATE TABLE bench_notification (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        kind varchar NOT NULL,
        expires_at timestamp NOT NULL,
        read_at timestamp NOT NULL
    )
    """
    )

    # bench_space
    await cur.execute(
        """
    CREATE TABLE bench_space (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        name varchar NOT NULL,
        text jsonb,
        order_key varchar NOT NULL,
        policies jsonb[],
        dock jsonb NOT NULL
    )
    """
    )

    # bench_handle
    await cur.execute(
        """
    CREATE TABLE bench_handle (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        slug varchar NOT NULL
    )
    """
    )

    # bench_filecontent
    await cur.execute(
        """
    CREATE TABLE bench_filecontent (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        sha512 varchar NOT NULL,
        content_length bigint NOT NULL,
        content_type varchar NOT NULL,
        status varchar NOT NULL
    )
    """
    )

    # bench_organization
    await cur.execute(
        """
    CREATE TABLE bench_organization (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        slug varchar,
        name varchar NOT NULL,
        text jsonb
    )
    """
    )

    # bench_client
    await cur.execute(
        """
    CREATE TABLE bench_client (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        name varchar,
        device_name varchar NOT NULL,
        browser_name varchar,
        last_seen_at timestamp NOT NULL,
        logged_in_at timestamp,
        access_token varchar,
        main_space_ck uuid
    )
    """
    )

    # bench_environment
    await cur.execute(
        """
    CREATE TABLE bench_environment (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        name varchar,
        text jsonb,
        policies jsonb[]
    )
    """
    )

    # bench_notice
    await cur.execute(
        """
    CREATE TABLE bench_notice (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        kind varchar NOT NULL,
        type varchar NOT NULL,
        message varchar NOT NULL,
        path jsonb,
        properties_ptr jsonb[]
    )
    """
    )

    # bench_membership
    await cur.execute(
        """
    CREATE TABLE bench_membership (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        is_owner boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_store
    await cur.execute(
        """
    CREATE TABLE bench_store (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        kind varchar NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        handle varchar,
        username bytea,
        password bytea
    )
    """
    )

    # bench_bench
    await cur.execute(
        """
    CREATE TABLE bench_bench (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        slug varchar NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        policies jsonb[]
    )
    """
    )

    # bench_field
    await cur.execute(
        """
    CREATE TABLE bench_field (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        name varchar,
        order_key varchar,
        dynamic_key varchar,
        text jsonb,
        value_packed jsonb,
        primitive_type varchar,
        bench_type varchar,
        base_type_block_ck uuid,
        format_hint varchar,
        condition jsonb,
        length integer,
        precision integer,
        scale integer,
        is_array boolean NOT NULL DEFAULT false,
        is_required boolean NOT NULL DEFAULT false,
        is_secret boolean NOT NULL DEFAULT false,
        is_input boolean NOT NULL DEFAULT false,
        is_output boolean NOT NULL DEFAULT false,
        is_option boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_branch
    await cur.execute(
        """
    CREATE TABLE bench_branch (
        id uuid NOT NULL PRIMARY KEY,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        name varchar,
        text jsonb,
        policies jsonb[]
    )
    """
    )

    # bench_trigger
    await cur.execute(
        """
    CREATE TABLE bench_trigger (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        type varchar NOT NULL,
        name varchar,
        active boolean NOT NULL DEFAULT true,
        schedule jsonb,
        signal_block_ck uuid
    )
    """
    )

    # bench_query
    await cur.execute(
        """
        ALTER TABLE bench_query    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_query_bench_idx_package_deleted_at ON bench_query USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_query_bench_idx_package_archived_at ON bench_query USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_query    
        ADD CONSTRAINT bench_query_bench_check_one_parent CHECK ((parent_block_id IS NOT NULL))
    """
    )

    # bench_server
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_server_bench_idx_external_id ON bench_server USING BTREE (external_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_server_bench_idx_deleted_at ON bench_server USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_server_bench_idx_archived_at ON bench_server USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ADD CONSTRAINT bench_server_bench_idx_external_id UNIQUE USING INDEX bench_server_bench_idx_external_id,
        ADD CONSTRAINT bench_server_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE bench_user    
        ADD COLUMN main_handle_id uuid REFERENCES bench_handle ON DELETE SET NULL,
        ADD COLUMN main_bench_id uuid REFERENCES bench_bench ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_user_bench_idx_slug ON bench_user USING BTREE (slug)"
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_user_bench_idx_email ON bench_user USING BTREE (email)"
    )
    await cur.execute(
        "CREATE INDEX bench_user_bench_idx_deleted_at ON bench_user USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_user_bench_idx_archived_at ON bench_user USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_user    
        ADD CONSTRAINT bench_user_bench_idx_slug UNIQUE USING INDEX bench_user_bench_idx_slug,
        ADD CONSTRAINT bench_user_bench_idx_email UNIQUE USING INDEX bench_user_bench_idx_email
    """
    )

    # bench_cache
    await cur.execute(
        """
        ALTER TABLE bench_cache    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_cache_bench_idx_deleted_at ON bench_cache USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_cache_bench_idx_archived_at ON bench_cache USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_cache    
        ADD CONSTRAINT bench_cache_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
    """
    )

    # bench_identity
    await cur.execute(
        """
        ALTER TABLE bench_identity    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_membership_id uuid REFERENCES bench_membership ON DELETE CASCADE,
        ADD COLUMN parent_user_id uuid REFERENCES bench_user ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_identity_bench_idx_package_deleted_at ON bench_identity USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_identity_bench_idx_package_archived_at ON bench_identity USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_identity    
        ADD CONSTRAINT bench_identity_bench_check_one_parent CHECK ((parent_block_id IS NOT NULL) OR (parent_membership_id IS NOT NULL) OR (parent_user_id IS NOT NULL))
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE bench_role    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_membership_id uuid REFERENCES bench_membership ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_role_bench_idx_package_deleted_at ON bench_role USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_role_bench_idx_package_archived_at ON bench_role USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_role    
        ADD CONSTRAINT bench_role_bench_check_one_parent CHECK ((parent_block_id IS NOT NULL) OR (parent_membership_id IS NOT NULL))
    """
    )

    # bench_skip
    await cur.execute(
        """
        ALTER TABLE bench_skip    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_skip_bench_idx_package_deleted_at ON bench_skip USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_skip_bench_idx_package_archived_at ON bench_skip USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_skip    
        ADD CONSTRAINT bench_skip_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL) OR (parent_block_id IS NOT NULL))
    """
    )

    # bench_badge
    await cur.execute(
        """
        ALTER TABLE bench_badge    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_badge_bench_idx_link_token ON bench_badge USING BTREE (link_token)"
    )
    await cur.execute(
        "CREATE INDEX bench_badge_bench_idx_package_deleted_at ON bench_badge USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_badge_bench_idx_package_archived_at ON bench_badge USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_badge    
        ADD CONSTRAINT bench_badge_bench_idx_link_token UNIQUE USING INDEX bench_badge_bench_idx_link_token,
        ADD CONSTRAINT bench_badge_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL) OR (parent_block_id IS NOT NULL))
    """
    )

    # bench_link
    await cur.execute(
        """
        ALTER TABLE bench_link    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_link_bench_idx_package_deleted_at ON bench_link USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_link_bench_idx_package_archived_at ON bench_link USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_link    
        ADD CONSTRAINT bench_link_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL) OR (parent_block_id IS NOT NULL))
    """
    )

    # bench_drive
    await cur.execute(
        """
        ALTER TABLE bench_drive    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_drive_bench_idx_handle ON bench_drive USING BTREE (handle)"
    )
    await cur.execute(
        "CREATE INDEX bench_drive_bench_idx_deleted_at ON bench_drive USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_drive_bench_idx_archived_at ON bench_drive USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_drive    
        ADD CONSTRAINT bench_drive_bench_idx_handle UNIQUE USING INDEX bench_drive_bench_idx_handle,
        ADD CONSTRAINT bench_drive_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
    """
    )

    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_block_bench_idx_package_deleted_at ON bench_block USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_block_bench_idx_package_archived_at ON bench_block USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ADD CONSTRAINT bench_block_bench_check_one_parent CHECK ((parent_block_id IS NOT NULL) OR (parent_package_id IS NOT NULL))
    """
    )

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ADD COLUMN parent_space_id uuid REFERENCES bench_space ON DELETE CASCADE,
        ADD COLUMN parent_view_id uuid REFERENCES bench_view ON DELETE CASCADE,
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_view_bench_idx_package_deleted_at ON bench_view USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_view_bench_idx_package_archived_at ON bench_view USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ADD CONSTRAINT bench_view_bench_check_one_parent CHECK ((parent_space_id IS NOT NULL) OR (parent_view_id IS NOT NULL) OR (parent_block_id IS NOT NULL))
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE bench_package    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE,
        ADD COLUMN base_package_id uuid REFERENCES bench_package ON DELETE SET NULL,
        ADD COLUMN environment_id uuid NOT NULL REFERENCES bench_environment ON DELETE SET NULL,
        ADD COLUMN branch_id uuid NOT NULL REFERENCES bench_branch ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_package_bench_idx_parent_bench_id_slug ON bench_package USING BTREE (parent_bench_id, slug)"
    )
    await cur.execute(
        "CREATE INDEX bench_package_bench_idx_deleted_at ON bench_package USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_package_bench_idx_archived_at ON bench_package USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_package    
        ADD CONSTRAINT bench_package_bench_idx_parent_bench_id_slug UNIQUE USING INDEX bench_package_bench_idx_parent_bench_id_slug,
        ADD CONSTRAINT bench_package_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
    """
    )

    # bench_tag
    await cur.execute(
        """
        ALTER TABLE bench_tag    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_field_id uuid REFERENCES bench_field ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_tag_bench_idx_package_deleted_at ON bench_tag USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_tag_bench_idx_package_archived_at ON bench_tag USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_tag    
        ADD CONSTRAINT bench_tag_bench_check_one_parent CHECK ((parent_block_id IS NOT NULL) OR (parent_field_id IS NOT NULL))
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        ADD COLUMN parent_user_id uuid REFERENCES bench_user ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_notification_bench_idx_deleted_at ON bench_notification USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_notification_bench_idx_archived_at ON bench_notification USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        ADD CONSTRAINT bench_notification_bench_check_one_parent CHECK ((parent_user_id IS NOT NULL))
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_space_bench_idx_package_deleted_at ON bench_space USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_space_bench_idx_package_archived_at ON bench_space USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ADD CONSTRAINT bench_space_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )

    # bench_handle
    await cur.execute(
        """
        ALTER TABLE bench_handle    
        ADD COLUMN parent_user_id uuid REFERENCES bench_user ON DELETE CASCADE,
        ADD COLUMN parent_organization_id uuid REFERENCES bench_organization ON DELETE CASCADE,
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_handle_bench_idx_slug ON bench_handle USING BTREE (slug)"
    )
    await cur.execute(
        "CREATE INDEX bench_handle_bench_idx_deleted_at ON bench_handle USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_handle_bench_idx_archived_at ON bench_handle USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_handle    
        ADD CONSTRAINT bench_handle_bench_idx_slug UNIQUE USING INDEX bench_handle_bench_idx_slug,
        ADD CONSTRAINT bench_handle_bench_check_one_parent CHECK ((parent_user_id IS NOT NULL) OR (parent_organization_id IS NOT NULL) OR (parent_bench_id IS NOT NULL))
    """
    )

    # bench_filecontent
    await cur.execute(
        """
        ALTER TABLE bench_filecontent    
        ADD COLUMN parent_drive_id uuid REFERENCES bench_drive ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_filecontent_bench_idx_deleted_at ON bench_filecontent USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_filecontent_bench_idx_archived_at ON bench_filecontent USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_filecontent    
        ADD CONSTRAINT bench_filecontent_bench_check_one_parent CHECK ((parent_drive_id IS NOT NULL))
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE bench_organization    
        ADD COLUMN main_handle_id uuid REFERENCES bench_handle ON DELETE SET NULL,
        ADD COLUMN main_bench_id uuid REFERENCES bench_bench ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_organization_bench_idx_slug ON bench_organization USING BTREE (slug)"
    )
    await cur.execute(
        "CREATE INDEX bench_organization_bench_idx_deleted_at ON bench_organization USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_organization_bench_idx_archived_at ON bench_organization USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_organization    
        ADD CONSTRAINT bench_organization_bench_idx_slug UNIQUE USING INDEX bench_organization_bench_idx_slug
    """
    )

    # bench_client
    await cur.execute(
        """
        ALTER TABLE bench_client    
        ADD COLUMN parent_user_id uuid REFERENCES bench_user ON DELETE CASCADE,
        ADD COLUMN parent_server_id uuid REFERENCES bench_server ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_client_bench_idx_access_token ON bench_client USING BTREE (access_token)"
    )
    await cur.execute(
        "CREATE INDEX bench_client_bench_idx_deleted_at ON bench_client USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_client_bench_idx_archived_at ON bench_client USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_client    
        ADD CONSTRAINT bench_client_bench_idx_access_token UNIQUE USING INDEX bench_client_bench_idx_access_token,
        ADD CONSTRAINT bench_client_bench_check_one_parent CHECK ((parent_user_id IS NOT NULL) OR (parent_server_id IS NOT NULL))
    """
    )

    # bench_environment
    await cur.execute(
        """
        ALTER TABLE bench_environment    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE,
        ADD COLUMN store_id uuid NOT NULL REFERENCES bench_store ON DELETE SET NULL,
        ADD COLUMN index_store_id uuid NOT NULL REFERENCES bench_store ON DELETE SET NULL,
        ADD COLUMN drive_id uuid NOT NULL REFERENCES bench_drive ON DELETE SET NULL,
        ADD COLUMN cache_id uuid NOT NULL REFERENCES bench_cache ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE INDEX bench_environment_bench_idx_deleted_at ON bench_environment USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_environment_bench_idx_archived_at ON bench_environment USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_environment    
        ADD CONSTRAINT bench_environment_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
    """
    )

    # bench_notice
    await cur.execute(
        """
        ALTER TABLE bench_notice    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_notice_bench_idx_package_deleted_at ON bench_notice USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_notice_bench_idx_package_archived_at ON bench_notice USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_notice    
        ADD CONSTRAINT bench_notice_bench_check_one_parent CHECK ((parent_block_id IS NOT NULL) OR (parent_package_id IS NOT NULL))
    """
    )

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE bench_membership    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE,
        ADD COLUMN parent_organization_id uuid REFERENCES bench_organization ON DELETE CASCADE,
        ADD COLUMN user_id uuid NOT NULL REFERENCES bench_user ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE INDEX bench_membership_bench_idx_deleted_at ON bench_membership USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_membership_bench_idx_archived_at ON bench_membership USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_membership    
        ADD CONSTRAINT bench_membership_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL) OR (parent_organization_id IS NOT NULL))
    """
    )

    # bench_store
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_store_bench_idx_handle ON bench_store USING BTREE (handle)"
    )
    await cur.execute(
        "CREATE INDEX bench_store_bench_idx_deleted_at ON bench_store USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_store_bench_idx_archived_at ON bench_store USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ADD CONSTRAINT bench_store_bench_idx_handle UNIQUE USING INDEX bench_store_bench_idx_handle,
        ADD CONSTRAINT bench_store_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
    """
    )

    # bench_bench
    await cur.execute(
        """
        ALTER TABLE bench_bench    
        ADD COLUMN main_handle_id uuid REFERENCES bench_handle ON DELETE SET NULL,
        ADD COLUMN owner_user_id uuid REFERENCES bench_user ON DELETE SET NULL,
        ADD COLUMN owner_organization_id uuid REFERENCES bench_organization ON DELETE SET NULL,
        ADD COLUMN main_package_id uuid REFERENCES bench_package ON DELETE SET NULL,
        ADD COLUMN main_environment_id uuid REFERENCES bench_environment ON DELETE SET NULL,
        ADD COLUMN main_branch_id uuid REFERENCES bench_branch ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_bench_bench_idx_slug ON bench_bench USING BTREE (slug)"
    )
    await cur.execute(
        "CREATE INDEX bench_bench_bench_idx_deleted_at ON bench_bench USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_bench_bench_idx_archived_at ON bench_bench USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_bench    
        ADD CONSTRAINT bench_bench_bench_idx_slug UNIQUE USING INDEX bench_bench_bench_idx_slug
    """
    )

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_field_bench_idx_package_deleted_at ON bench_field USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_field_bench_idx_package_archived_at ON bench_field USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ADD CONSTRAINT bench_field_bench_check_one_parent CHECK ((parent_block_id IS NOT NULL))
    """
    )

    # bench_branch
    await cur.execute(
        """
        ALTER TABLE bench_branch    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE,
        ADD COLUMN main_package_id uuid REFERENCES bench_package ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE INDEX bench_branch_bench_idx_deleted_at ON bench_branch USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_branch_bench_idx_archived_at ON bench_branch USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_branch    
        ADD CONSTRAINT bench_branch_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
    """
    )

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN package_id uuid NOT NULL REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_trigger_bench_idx_package_deleted_at ON bench_trigger USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_trigger_bench_idx_package_archived_at ON bench_trigger USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ADD CONSTRAINT bench_trigger_bench_check_one_parent CHECK ((parent_block_id IS NOT NULL))
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_migration
    await cur.execute(
        """
    CREATE TABLE bench_migration (
        id integer NOT NULL PRIMARY KEY,
        version varchar NOT NULL,
        has_global boolean NOT NULL,
        has_local boolean NOT NULL,
        applied_at timestamp
    )
    """
    )

    # bench_record_ephemeral
    await cur.execute(
        """
    CREATE TABLE bench_record_ephemeral (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL DEFAULT now(),
        updated_at timestamp NOT NULL DEFAULT now(),
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL DEFAULT now(),
        block_key varchar NOT NULL,
        block_ck uuid NOT NULL,
        block_id uuid NOT NULL,
        value_packed jsonb
    )
    """
    )

    # bench_run
    await cur.execute(
        """
    CREATE TABLE bench_run (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        server_id uuid,
        block_ck uuid,
        scheduled_at timestamp,
        started_at timestamp,
        terminated_at timestamp,
        status varchar NOT NULL,
        inputs_packed jsonb,
        outputs_packed jsonb,
        value_packed jsonb,
        error jsonb
    )
    """
    )

    # bench_session
    await cur.execute(
        """
    CREATE TABLE bench_session (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        parent_package_id uuid,
        package_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        last_changed_at timestamp,
        server_id uuid,
        server_process_id varchar,
        trigger_type varchar,
        trigger_id uuid,
        opened_at timestamp,
        closed_at timestamp
    )
    """
    )

    # bench_signal
    await cur.execute(
        """
    CREATE TABLE bench_signal (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        parent_package_id uuid,
        package_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL,
        type_block_ck uuid,
        value_packed jsonb
    )
    """
    )

    # bench_pause
    await cur.execute(
        """
    CREATE TABLE bench_pause (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        last_edited_at timestamp NOT NULL
    )
    """
    )

    # bench_record_ephemeral
    await cur.execute(
        "CREATE INDEX bench_record_ephemeral_bench_idx_block_key_deleted_at ON bench_record_ephemeral USING BTREE (block_key, deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_record_ephemeral_bench_idx_block_key_archive_at ON bench_record_ephemeral USING BTREE (archived_at, block_key)"
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_record_ephemeral_bench_idx_ck_block_key ON bench_record_ephemeral USING BTREE (block_key, ck)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_record_ephemeral    
        ADD CONSTRAINT bench_record_ephemeral_bench_idx_ck_block_key UNIQUE USING INDEX bench_record_ephemeral_bench_idx_ck_block_key
    """
    )

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ADD COLUMN parent_session_id uuid REFERENCES bench_session ON DELETE CASCADE,
        ADD COLUMN parent_run_id uuid REFERENCES bench_run ON DELETE CASCADE,
        ADD COLUMN session_id uuid NOT NULL REFERENCES bench_session ON DELETE CASCADE,
        ADD COLUMN root_run_id uuid REFERENCES bench_run ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_run_bench_idx_session_id ON bench_run USING BTREE (session_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_run_bench_idx_root_run_id ON bench_run USING BTREE (root_run_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_run_bench_idx_server_id ON bench_run USING BTREE (server_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_run_bench_idx_block_ck ON bench_run USING BTREE (block_ck)"
    )
    await cur.execute("CREATE INDEX bench_run_bench_idx_status ON bench_run USING BTREE (status)")
    await cur.execute(
        "CREATE INDEX bench_run_bench_idx_package_deleted_at ON bench_run USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_run_bench_idx_package_archived_at ON bench_run USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ADD CONSTRAINT bench_run_bench_check_one_parent CHECK ((parent_session_id IS NOT NULL) OR (parent_run_id IS NOT NULL))
    """
    )

    # bench_session
    await cur.execute(
        "CREATE INDEX bench_session_bench_idx_package_deleted_at ON bench_session USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_session_bench_idx_package_archived_at ON bench_session USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_session    
        ADD CONSTRAINT bench_session_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )

    # bench_signal
    await cur.execute(
        "CREATE INDEX bench_signal_bench_idx_type_block_ck ON bench_signal USING BTREE (type_block_ck)"
    )
    await cur.execute(
        "CREATE INDEX bench_signal_bench_idx_package_deleted_at ON bench_signal USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_signal_bench_idx_package_archived_at ON bench_signal USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_signal    
        ADD CONSTRAINT bench_signal_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )

    # bench_pause
    await cur.execute(
        """
        ALTER TABLE bench_pause    
        ADD COLUMN parent_run_id uuid REFERENCES bench_run ON DELETE CASCADE,
        ADD COLUMN session_id uuid NOT NULL REFERENCES bench_session ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_pause_bench_idx_package_deleted_at ON bench_pause USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_pause_bench_idx_package_archived_at ON bench_pause USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_pause    
        ADD CONSTRAINT bench_pause_bench_check_one_parent CHECK ((parent_run_id IS NOT NULL))
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
