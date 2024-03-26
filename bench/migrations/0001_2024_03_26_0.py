# This migration was automatically generated on 2024.03.26. Edit as needed.
import psycopg

ID = 1
VERSION = "2024.03.26.0"
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
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        slug varchar NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        icon jsonb,
        owner_user_id uuid,
        region smallint NOT NULL,
        encryption_key bytea NOT NULL,
        policies jsonb[] NOT NULL
    )
    """
    )

    # bench_environment
    await cur.execute(
        """
    CREATE TABLE bench_environment (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar,
        text jsonb,
        icon jsonb,
        policies jsonb[] NOT NULL
    )
    """
    )

    # bench_branch
    await cur.execute(
        """
    CREATE TABLE bench_branch (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar,
        slug varchar,
        text jsonb,
        icon jsonb,
        policies jsonb[] NOT NULL,
        main_package_id uuid
    )
    """
    )

    # bench_package
    await cur.execute(
        """
    CREATE TABLE bench_package (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        slug varchar,
        text jsonb,
        icon jsonb,
        policies jsonb[] NOT NULL,
        paused_at timestamp,
        bases_package_id uuid[]
    )
    """
    )

    # bench_dependency
    await cur.execute(
        """
    CREATE TABLE bench_dependency (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        scopes_ck uuid[] NOT NULL,
        scopes_bench_id uuid[] NOT NULL,
        dependency_package_id uuid NOT NULL,
        dependency_scopes_ck uuid[] NOT NULL,
        dependency_scopes_bench_id uuid[] NOT NULL
    )
    """
    )

    # bench_upgrade
    await cur.execute(
        """
    CREATE TABLE bench_upgrade (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar,
        title varchar,
        text jsonb
    )
    """
    )

    # bench_space
    await cur.execute(
        """
    CREATE TABLE bench_space (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        order_key varchar NOT NULL,
        policies jsonb[],
        focus jsonb
    )
    """
    )

    # bench_link
    await cur.execute(
        """
    CREATE TABLE bench_link (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        reference_ck uuid,
        reference_type smallint,
        reference_bench_id uuid,
        reference_base_ck uuid,
        reference_base_bench_id uuid,
        order_key varchar
    )
    """
    )

    # bench_notice
    await cur.execute(
        """
    CREATE TABLE bench_notice (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        kind smallint NOT NULL,
        type smallint NOT NULL,
        message varchar,
        path jsonb,
        properties_ptr jsonb[]
    )
    """
    )

    # bench_block
    await cur.execute(
        """
    CREATE TABLE bench_block (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        type smallint NOT NULL DEFAULT 2,
        name varchar,
        order_key varchar NOT NULL DEFAULT 'a0'::character varying,
        visibility smallint NOT NULL DEFAULT 10,
        policies jsonb[] NOT NULL,
        bases_ck uuid[],
        bases_bench_id uuid[],
        builtin_base jsonb,
        dynamic_key varchar,
        text jsonb,
        value_packed jsonb,
        secret_value_packed bytea,
        code jsonb,
        icon jsonb,
        reference_ck uuid,
        reference_bench_id uuid,
        delegated_policies jsonb[] NOT NULL,
        is_page boolean NOT NULL DEFAULT false,
        is_module boolean NOT NULL DEFAULT false,
        is_unique boolean NOT NULL DEFAULT false,
        is_intrinsic boolean NOT NULL DEFAULT false,
        is_protocol boolean NOT NULL DEFAULT false,
        is_method boolean NOT NULL DEFAULT false,
        paused_at timestamp
    )
    """
    )

    # bench_trigger
    await cur.execute(
        """
    CREATE TABLE bench_trigger (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        type smallint NOT NULL,
        name varchar,
        active boolean NOT NULL DEFAULT true,
        schedule jsonb,
        signal_ck uuid,
        signal_bench_id uuid
    )
    """
    )

    # bench_field
    await cur.execute(
        """
    CREATE TABLE bench_field (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar,
        order_key varchar NOT NULL DEFAULT 'a0'::character varying,
        dynamic_key varchar,
        text jsonb,
        icon jsonb,
        value_packed jsonb,
        primitive_type smallint,
        node_type smallint,
        struct_type smallint,
        base_type_ck uuid,
        base_type_bench_id uuid,
        visibility smallint NOT NULL DEFAULT 10,
        format_hint smallint,
        condition jsonb,
        length integer,
        precision integer,
        scale integer,
        default_packed jsonb,
        is_list boolean NOT NULL DEFAULT false,
        is_required boolean NOT NULL DEFAULT false,
        is_secret boolean NOT NULL DEFAULT false,
        is_input boolean NOT NULL DEFAULT false,
        is_output boolean NOT NULL DEFAULT false,
        is_option boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_query
    await cur.execute(
        """
    CREATE TABLE bench_query (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar,
        order_key varchar NOT NULL DEFAULT 'a0'::character varying,
        node_type smallint NOT NULL,
        base_ck uuid,
        base_bench_id uuid,
        filter jsonb,
        sort jsonb[]
    )
    """
    )

    # bench_view
    await cur.execute(
        """
    CREATE TABLE bench_view (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        type smallint NOT NULL,
        name varchar,
        title varchar,
        text jsonb,
        order_key varchar NOT NULL DEFAULT 'a0'::character varying,
        icon jsonb,
        value_packed jsonb,
        node_ck uuid,
        node_type smallint,
        node_bench_id uuid,
        node_base_ck uuid,
        node_base_bench_id uuid,
        variant smallint,
        font jsonb,
        position jsonb,
        size jsonb,
        margin jsonb,
        padding jsonb,
        orientation smallint,
        alignment smallint,
        selection jsonb,
        focus jsonb,
        is_visible boolean DEFAULT true,
        is_disabled boolean DEFAULT false,
        is_loading boolean DEFAULT false,
        is_input boolean DEFAULT false,
        is_secret boolean DEFAULT false
    )
    """
    )

    # bench_badge
    await cur.execute(
        """
    CREATE TABLE bench_badge (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar,
        delegated_policies jsonb[] NOT NULL,
        expires_at timestamp,
        key bytea,
        key_hash bytea,
        password bytea,
        password_hash bytea
    )
    """
    )

    # bench_role
    await cur.execute(
        """
    CREATE TABLE bench_role (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        type_ck uuid NOT NULL,
        type_bench_id uuid NOT NULL
    )
    """
    )

    # bench_identity
    await cur.execute(
        """
    CREATE TABLE bench_identity (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        type_ck uuid NOT NULL,
        type_bench_id uuid NOT NULL
    )
    """
    )

    # bench_membership
    await cur.execute(
        """
    CREATE TABLE bench_membership (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        user_id uuid NOT NULL,
        is_owner boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_invite
    await cur.execute(
        """
    CREATE TABLE bench_invite (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        user_id uuid,
        user_email varchar,
        is_owner boolean NOT NULL DEFAULT false,
        roles_ck uuid[],
        roles_bench_id uuid[]
    )
    """
    )

    # bench_server
    await cur.execute(
        """
    CREATE TABLE bench_server (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        region smallint NOT NULL DEFAULT 1,
        tenancy smallint NOT NULL DEFAULT 3,
        status smallint NOT NULL DEFAULT 1,
        profile smallint NOT NULL,
        version varchar,
        is_paused boolean NOT NULL DEFAULT true,
        current_profile smallint,
        current_version varchar,
        last_active_at timestamp,
        last_bumped_at timestamp
    )
    """
    )

    # bench_store
    await cur.execute(
        """
    CREATE TABLE bench_store (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        region smallint NOT NULL DEFAULT 1,
        tenancy smallint NOT NULL DEFAULT 3,
        status smallint NOT NULL DEFAULT 1,
        kind smallint NOT NULL,
        engine smallint NOT NULL,
        version varchar,
        host varchar,
        database varchar,
        schema varchar,
        main_credential bytea
    )
    """
    )

    # bench_drive
    await cur.execute(
        """
    CREATE TABLE bench_drive (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        region smallint NOT NULL DEFAULT 1,
        tenancy smallint NOT NULL DEFAULT 3,
        status smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_cache
    await cur.execute(
        """
    CREATE TABLE bench_cache (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar NOT NULL,
        text jsonb,
        region smallint NOT NULL DEFAULT 1,
        tenancy smallint NOT NULL DEFAULT 3,
        status smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_filecontent
    await cur.execute(
        """
    CREATE TABLE bench_filecontent (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        sha512 varchar NOT NULL,
        size bigint NOT NULL,
        type varchar NOT NULL,
        status smallint NOT NULL,
        retention smallint NOT NULL,
        expires_at timestamp
    )
    """
    )

    # bench_handle
    await cur.execute(
        """
    CREATE TABLE bench_handle (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        slug varchar NOT NULL
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
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        slug varchar,
        name varchar,
        text jsonb,
        email varchar NOT NULL,
        icon jsonb,
        main_bench_id uuid,
        status smallint NOT NULL,
        password_salt bytea,
        password_hash bytea,
        last_logged_in_at timestamp,
        is_staff boolean NOT NULL DEFAULT false
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
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        slug varchar,
        name varchar NOT NULL,
        text jsonb,
        icon jsonb,
        main_bench_id uuid,
        status smallint NOT NULL
    )
    """
    )

    # bench_client
    await cur.execute(
        """
    CREATE TABLE bench_client (
        id uuid NOT NULL PRIMARY KEY,
        bench_id uuid,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        created_by_run_id uuid,
        updated_by_user_id uuid,
        updated_by_run_id uuid,
        set_properties integer[] NOT NULL,
        name varchar,
        device_name varchar,
        device_type varchar,
        operating_system varchar,
        browser_name varchar,
        browser_version varchar,
        access_token varchar,
        last_seen_at timestamp NOT NULL,
        logged_in_at timestamp,
        main_space_ck uuid,
        main_space_bench_id uuid
    )
    """
    )

    # bench_bench
    await cur.execute(
        """
        ALTER TABLE bench_bench    
        ADD COLUMN main_handle_id uuid REFERENCES bench_handle ON DELETE SET NULL,
        ADD COLUMN owner_organization_id uuid REFERENCES bench_organization ON DELETE SET NULL,
        ADD COLUMN main_environment_id uuid REFERENCES bench_environment ON DELETE SET NULL,
        ADD COLUMN main_branch_id uuid REFERENCES bench_branch ON DELETE SET NULL,
        ADD COLUMN published_branch_id uuid REFERENCES bench_branch ON DELETE SET NULL
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

    # bench_environment
    await cur.execute(
        """
        ALTER TABLE bench_environment    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE,
        ADD COLUMN server_id uuid NOT NULL REFERENCES bench_server ON DELETE SET NULL,
        ADD COLUMN store_id uuid NOT NULL REFERENCES bench_store ON DELETE SET NULL,
        ADD COLUMN search_store_id uuid NOT NULL REFERENCES bench_store ON DELETE SET NULL,
        ADD COLUMN analytics_store_id uuid REFERENCES bench_store ON DELETE SET NULL,
        ADD COLUMN drive_id uuid NOT NULL REFERENCES bench_drive ON DELETE SET NULL,
        ADD COLUMN cache_id uuid REFERENCES bench_cache ON DELETE SET NULL
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

    # bench_branch
    await cur.execute(
        """
        ALTER TABLE bench_branch    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_branch_bench_idx_parent_bench_id_slug ON bench_branch USING BTREE (parent_bench_id, slug)"
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
        ADD CONSTRAINT bench_branch_bench_idx_parent_bench_id_slug UNIQUE USING INDEX bench_branch_bench_idx_parent_bench_id_slug,
        ADD CONSTRAINT bench_branch_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
    """
    )

    # bench_package
    await cur.execute(
        """
        ALTER TABLE bench_package    
        ADD COLUMN parent_bench_id uuid REFERENCES bench_bench ON DELETE CASCADE,
        ADD COLUMN environment_id uuid NOT NULL REFERENCES bench_environment ON DELETE SET NULL
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

    # bench_dependency
    await cur.execute(
        """
        ALTER TABLE bench_dependency    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_dependency_bench_idx_package_deleted_at ON bench_dependency USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_dependency_bench_idx_package_archived_at ON bench_dependency USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_dependency    
        ADD CONSTRAINT bench_dependency_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL) OR (parent_block_id IS NOT NULL))
    """
    )

    # bench_upgrade
    await cur.execute(
        """
        ALTER TABLE bench_upgrade    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_upgrade_bench_idx_package_deleted_at ON bench_upgrade USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_upgrade_bench_idx_package_archived_at ON bench_upgrade USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_upgrade    
        ADD CONSTRAINT bench_upgrade_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )

    # bench_space
    await cur.execute(
        """
        ALTER TABLE bench_space    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE
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

    # bench_link
    await cur.execute(
        """
        ALTER TABLE bench_link    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE
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

    # bench_notice
    await cur.execute(
        """
        ALTER TABLE bench_notice    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE
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

    # bench_block
    await cur.execute(
        """
        ALTER TABLE bench_block    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE
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

    # bench_trigger
    await cur.execute(
        """
        ALTER TABLE bench_trigger    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE
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

    # bench_field
    await cur.execute(
        """
        ALTER TABLE bench_field    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE
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

    # bench_query
    await cur.execute(
        """
        ALTER TABLE bench_query    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE
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

    # bench_view
    await cur.execute(
        """
        ALTER TABLE bench_view    
        ADD COLUMN parent_space_id uuid REFERENCES bench_space ON DELETE CASCADE,
        ADD COLUMN parent_view_id uuid REFERENCES bench_view ON DELETE CASCADE,
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE
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

    # bench_badge
    await cur.execute(
        """
        ALTER TABLE bench_badge    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE,
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_badge_bench_idx_key ON bench_badge USING BTREE (key)"
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_badge_bench_idx_key_hash ON bench_badge USING BTREE (key_hash)"
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
        ADD CONSTRAINT bench_badge_bench_idx_key UNIQUE USING INDEX bench_badge_bench_idx_key,
        ADD CONSTRAINT bench_badge_bench_idx_key_hash UNIQUE USING INDEX bench_badge_bench_idx_key_hash,
        ADD CONSTRAINT bench_badge_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL) OR (parent_block_id IS NOT NULL))
    """
    )

    # bench_role
    await cur.execute(
        """
        ALTER TABLE bench_role    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_membership_id uuid REFERENCES bench_membership ON DELETE CASCADE
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

    # bench_identity
    await cur.execute(
        """
        ALTER TABLE bench_identity    
        ADD COLUMN parent_block_id uuid REFERENCES bench_block ON DELETE CASCADE,
        ADD COLUMN parent_membership_id uuid REFERENCES bench_membership ON DELETE CASCADE,
        ADD COLUMN parent_user_id uuid REFERENCES bench_user ON DELETE CASCADE
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

    # bench_membership
    await cur.execute(
        """
        ALTER TABLE bench_membership    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_membership_bench_idx_package_deleted_at ON bench_membership USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_membership_bench_idx_package_archived_at ON bench_membership USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_membership    
        ADD CONSTRAINT bench_membership_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )

    # bench_invite
    await cur.execute(
        """
        ALTER TABLE bench_invite    
        ADD COLUMN parent_package_id uuid REFERENCES bench_package ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_invite_bench_idx_package_deleted_at ON bench_invite USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_invite_bench_idx_package_archived_at ON bench_invite USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_invite    
        ADD CONSTRAINT bench_invite_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
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
        "CREATE INDEX bench_server_bench_idx_deleted_at ON bench_server USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_server_bench_idx_archived_at ON bench_server USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_server    
        ADD CONSTRAINT bench_server_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
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
        "CREATE INDEX bench_store_bench_idx_deleted_at ON bench_store USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_store_bench_idx_archived_at ON bench_store USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_store    
        ADD CONSTRAINT bench_store_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
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
        "CREATE INDEX bench_drive_bench_idx_deleted_at ON bench_drive USING BTREE (deleted_at)"
    )
    await cur.execute(
        "CREATE INDEX bench_drive_bench_idx_archived_at ON bench_drive USING BTREE (archived_at)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_drive    
        ADD CONSTRAINT bench_drive_bench_check_one_parent CHECK ((parent_bench_id IS NOT NULL))
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

    # bench_filecontent
    await cur.execute(
        """
        ALTER TABLE bench_filecontent    
        ADD COLUMN parent_drive_id uuid REFERENCES bench_drive ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE UNIQUE INDEX bench_filecontent_bench_idx_parent_drive_id_sha512 ON bench_filecontent USING BTREE (parent_drive_id, sha512)"
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
        ADD CONSTRAINT bench_filecontent_bench_idx_parent_drive_id_sha512 UNIQUE USING INDEX bench_filecontent_bench_idx_parent_drive_id_sha512,
        ADD CONSTRAINT bench_filecontent_bench_check_one_parent CHECK ((parent_drive_id IS NOT NULL))
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
        ADD CONSTRAINT bench_handle_bench_slug_is_slug CHECK (((slug)::text ~ \'^[a-z0-9-]{3,}$\'::text)),
        ADD CONSTRAINT bench_handle_bench_idx_slug UNIQUE USING INDEX bench_handle_bench_idx_slug,
        ADD CONSTRAINT bench_handle_bench_check_one_parent CHECK ((parent_user_id IS NOT NULL) OR (parent_organization_id IS NOT NULL) OR (parent_bench_id IS NOT NULL))
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE bench_user    
        ADD COLUMN main_handle_id uuid REFERENCES bench_handle ON DELETE SET NULL
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

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE bench_organization    
        ADD COLUMN main_handle_id uuid REFERENCES bench_handle ON DELETE SET NULL
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
        block_key varchar NOT NULL,
        block_ck uuid NOT NULL,
        block_id uuid NOT NULL,
        value_packed jsonb,
        secret_value_packed bytea
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
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        updated_by_user_id uuid,
        set_properties integer[] NOT NULL,
        server_id uuid,
        opened_at timestamp,
        closed_at timestamp,
        duration real,
        is_runtime boolean NOT NULL DEFAULT false,
        is_readonly boolean NOT NULL DEFAULT false
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
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        updated_by_user_id uuid,
        set_properties integer[] NOT NULL,
        root_ck uuid,
        root_bench_id uuid,
        root_base_ck uuid,
        root_base_bench_id uuid,
        server_id uuid,
        block_ck uuid,
        block_bench_id uuid,
        scheduled_at timestamp,
        started_at timestamp,
        terminated_at timestamp,
        duration real NOT NULL DEFAULT 0,
        status smallint NOT NULL,
        inputs_packed jsonb,
        inputs_secret_packed bytea,
        outputs_packed jsonb,
        outputs_secret_packed bytea,
        value_packed jsonb,
        value_secret_packed bytea,
        error jsonb
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
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        updated_by_user_id uuid,
        set_properties integer[] NOT NULL
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
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        updated_by_user_id uuid,
        set_properties integer[] NOT NULL,
        type_ck uuid,
        type_bench_id uuid,
        sender_ck uuid,
        sender_bench_id uuid,
        value_packed jsonb,
        secret_value_packed bytea
    )
    """
    )

    # bench_log
    await cur.execute(
        """
    CREATE TABLE bench_log (
        id uuid NOT NULL PRIMARY KEY,
        parent_package_id uuid,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        updated_by_user_id uuid,
        set_properties integer[] NOT NULL,
        kind smallint NOT NULL,
        level smallint NOT NULL,
        logger varchar,
        event varchar,
        message varchar,
        text jsonb,
        value_dynamic jsonb,
        request jsonb,
        session_ck uuid,
        session_bench_id uuid,
        run_ck uuid,
        run_bench_id uuid,
        run_base_ck uuid,
        run_base_bench_id uuid,
        block_ck uuid,
        block_bench_id uuid
    )
    """
    )

    # bench_notification
    await cur.execute(
        """
    CREATE TABLE bench_notification (
        id uuid NOT NULL PRIMARY KEY,
        ck uuid NOT NULL,
        parent_package_id uuid,
        package_id uuid NOT NULL,
        bench_id uuid NOT NULL,
        revision bigint NOT NULL DEFAULT 0,
        created_at timestamp NOT NULL,
        updated_at timestamp NOT NULL,
        deleted_at timestamp,
        archived_at timestamp,
        created_by_user_id uuid,
        updated_by_user_id uuid,
        set_properties integer[] NOT NULL,
        kind smallint NOT NULL,
        type_ck uuid,
        type_bench_id uuid,
        expires_at timestamp,
        read_at timestamp,
        sender_ck uuid,
        sender_bench_id uuid,
        title varchar,
        text jsonb,
        value_packed jsonb,
        secret_value_packed bytea
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

    # bench_session
    await cur.execute(
        """
        ALTER TABLE bench_session    
        ADD COLUMN created_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL,
        ADD COLUMN updated_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL
    """
    )
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

    # bench_run
    await cur.execute(
        """
        ALTER TABLE bench_run    
        ADD COLUMN parent_session_id uuid REFERENCES bench_session ON DELETE CASCADE,
        ADD COLUMN parent_run_id uuid REFERENCES bench_run ON DELETE CASCADE,
        ADD COLUMN created_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL,
        ADD COLUMN updated_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL,
        ADD COLUMN session_id uuid NOT NULL REFERENCES bench_session ON DELETE CASCADE
    """
    )
    await cur.execute(
        "CREATE INDEX bench_run_bench_idx_session_id ON bench_run USING BTREE (session_id)"
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

    # bench_pause
    await cur.execute(
        """
        ALTER TABLE bench_pause    
        ADD COLUMN parent_run_id uuid REFERENCES bench_run ON DELETE CASCADE,
        ADD COLUMN created_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL,
        ADD COLUMN updated_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL,
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

    # bench_signal
    await cur.execute(
        """
        ALTER TABLE bench_signal    
        ADD COLUMN created_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL,
        ADD COLUMN updated_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL
    """
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

    # bench_log
    await cur.execute(
        """
        ALTER TABLE bench_log    
        ADD COLUMN created_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL,
        ADD COLUMN updated_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE INDEX bench_log_bench_idx_package_deleted_at ON bench_log USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_log_bench_idx_package_archived_at ON bench_log USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_log    
        ADD CONSTRAINT bench_log_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )

    # bench_notification
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        ADD COLUMN created_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL,
        ADD COLUMN updated_by_run_id uuid REFERENCES bench_run ON DELETE SET NULL
    """
    )
    await cur.execute(
        "CREATE INDEX bench_notification_bench_idx_package_deleted_at ON bench_notification USING BTREE (deleted_at, package_id)"
    )
    await cur.execute(
        "CREATE INDEX bench_notification_bench_idx_package_archived_at ON bench_notification USING BTREE (archived_at, package_id)"
    )
    await cur.execute(
        """
        ALTER TABLE bench_notification    
        ADD CONSTRAINT bench_notification_bench_check_one_parent CHECK ((parent_package_id IS NOT NULL))
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
