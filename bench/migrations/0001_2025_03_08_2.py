# This migration was automatically generated on 2025.03.08. Edit as needed.
import psycopg

ID = 1
VERSION = "2025.03.08.2"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pgcrypto"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')

    # bench_migration
    await cur.execute(
        """
    CREATE TABLE "bench_migration" (
        "id" integer NOT NULL PRIMARY KEY,
        "version" varchar NOT NULL,
        "has_global" boolean NOT NULL,
        "has_regional" boolean NOT NULL,
        "has_local" boolean NOT NULL,
        "applied_at" timestamp
    )
    """
    )

    # bench_bench
    await cur.execute(
        """
    CREATE TABLE "bench_bench" (
        "id" uuid NOT NULL PRIMARY KEY,
        "bench_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "subnode_packed" jsonb,
        "main_handle_bench_id" uuid,
        "slug" varchar NOT NULL,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "region" smallint NOT NULL,
        "encryption_key" bytea NOT NULL,
        "status" smallint NOT NULL DEFAULT 20,
        "main_store_id" uuid,
        "main_package_id" uuid
    )
    """
    )

    # bench_handle
    await cur.execute(
        """
    CREATE TABLE "bench_handle" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "slug" varchar NOT NULL
    )
    """
    )

    # bench_user
    await cur.execute(
        """
    CREATE TABLE "bench_user" (
        "id" uuid NOT NULL PRIMARY KEY,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "slug" varchar,
        "name" varchar NOT NULL,
        "icon" jsonb,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "main_handle_bench_id" uuid,
        "email" varchar,
        "password_salt" bytea,
        "password_hash" bytea,
        "last_logged_in_at" timestamp,
        "is_staff" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_organization
    await cur.execute(
        """
    CREATE TABLE "bench_organization" (
        "id" uuid NOT NULL PRIMARY KEY,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "slug" varchar,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "main_handle_bench_id" uuid
    )
    """
    )

    # bench_team
    await cur.execute(
        """
    CREATE TABLE "bench_team" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "team_id" uuid,
        "organization_id" uuid,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "slug" varchar,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb
    )
    """
    )

    # bench_client
    await cur.execute(
        """
    CREATE TABLE "bench_client" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid,
        "user_id" uuid,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "device_type" varchar,
        "device_name" varchar,
        "operating_system" varchar,
        "browser_name" varchar,
        "browser_version" varchar,
        "place_id" varchar,
        "access_token" varchar,
        "seen_at" timestamp,
        "logged_in_at" timestamp,
        "space_id" uuid,
        "space_bench_id" uuid,
        "machine_id" uuid,
        "machine_bench_id" uuid
    )
    """
    )

    # bench_bench
    await cur.execute(
        """
        ALTER TABLE "bench_bench"    
        ADD COLUMN "main_handle_id" uuid REFERENCES bench_handle ON DELETE SET NULL
    """
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_bench_bench_idx_slug" ON "bench_bench" USING BTREE (slug)'
    )
    await cur.execute(
        """
        ALTER TABLE "bench_bench"    
        ADD CONSTRAINT "bench_bench_bench_idx_slug" UNIQUE USING INDEX bench_bench_bench_idx_slug
    """
    )

    # bench_handle
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_handle_bench_idx_slug" ON "bench_handle" USING BTREE (slug)'
    )
    await cur.execute(
        'CREATE INDEX "bench_handle_bench_idx_parent_id" ON "bench_handle" USING BTREE (parent_id) INCLUDE (id)'
    )
    await cur.execute(
        """
        ALTER TABLE "bench_handle"    
        ADD CONSTRAINT "bench_handle_bench_idx_slug" UNIQUE USING INDEX bench_handle_bench_idx_slug
    """
    )

    # bench_user
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        ADD COLUMN "main_bench_id" uuid REFERENCES bench_bench ON DELETE SET NULL,
        ADD COLUMN "main_handle_id" uuid REFERENCES bench_handle ON DELETE SET NULL
    """
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_user_bench_idx_slug" ON "bench_user" USING BTREE (slug)'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_user_bench_idx_email" ON "bench_user" USING BTREE (email)'
    )
    await cur.execute(
        """
        ALTER TABLE "bench_user"    
        ADD CONSTRAINT "bench_user_bench_idx_slug" UNIQUE USING INDEX bench_user_bench_idx_slug,
        ADD CONSTRAINT "bench_user_bench_idx_email" UNIQUE USING INDEX bench_user_bench_idx_email
    """
    )

    # bench_organization
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        ADD COLUMN "main_bench_id" uuid REFERENCES bench_bench ON DELETE SET NULL,
        ADD COLUMN "main_handle_id" uuid REFERENCES bench_handle ON DELETE SET NULL
    """
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_organization_bench_idx_slug" ON "bench_organization" USING BTREE (slug)'
    )
    await cur.execute(
        """
        ALTER TABLE "bench_organization"    
        ADD CONSTRAINT "bench_organization_bench_idx_slug" UNIQUE USING INDEX bench_organization_bench_idx_slug
    """
    )

    # bench_team
    await cur.execute(
        """
        ALTER TABLE "bench_team"    
        ADD COLUMN "main_bench_id" uuid REFERENCES bench_bench ON DELETE SET NULL
    """
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_team_bench_idx_slug" ON "bench_team" USING BTREE (slug)'
    )
    await cur.execute(
        'CREATE INDEX "bench_team_bench_idx_parent_id" ON "bench_team" USING BTREE (parent_id) INCLUDE (id)'
    )
    await cur.execute(
        """
        ALTER TABLE "bench_team"    
        ADD CONSTRAINT "bench_team_bench_idx_slug" UNIQUE USING INDEX bench_team_bench_idx_slug
    """
    )

    # bench_client
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_client_bench_idx_access_token" ON "bench_client" USING BTREE (access_token)'
    )
    await cur.execute(
        'CREATE INDEX "bench_client_bench_idx_parent_id" ON "bench_client" USING BTREE (parent_id) INCLUDE (id)'
    )
    await cur.execute(
        """
        ALTER TABLE "bench_client"    
        ADD CONSTRAINT "bench_client_bench_idx_access_token" UNIQUE USING INDEX bench_client_bench_idx_access_token
    """
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pgcrypto"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')

    # bench_migration
    await cur.execute(
        """
    CREATE TABLE "bench_migration" (
        "id" integer NOT NULL PRIMARY KEY,
        "version" varchar NOT NULL,
        "has_global" boolean NOT NULL,
        "has_regional" boolean NOT NULL,
        "has_local" boolean NOT NULL,
        "applied_at" timestamp
    )
    """
    )

    # bench_scaler
    await cur.execute(
        """
    CREATE TABLE "bench_scaler" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "strategy" smallint NOT NULL,
        "target_count" integer NOT NULL DEFAULT 0,
        "min_count" integer NOT NULL DEFAULT 0,
        "max_count" integer NOT NULL DEFAULT 16,
        "min_ready_count" integer NOT NULL DEFAULT 0,
        "is_active" boolean NOT NULL DEFAULT true,
        "is_main" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_store
    await cur.execute(
        """
    CREATE TABLE "bench_store" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 1,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "version" varchar NOT NULL,
        "target_version" varchar NOT NULL,
        "external_name" varchar,
        "external_id" varchar,
        "connection_uri" bytea
    )
    """
    )

    # bench_machine
    await cur.execute(
        """
    CREATE TABLE "bench_machine" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 1,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "scaler_id" uuid,
        "scaler_bench_id" uuid,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "version" varchar NOT NULL,
        "target_version" varchar NOT NULL,
        "external_name" varchar,
        "external_id" varchar,
        "connection_uri" bytea,
        "client_id" uuid,
        "cpu" real NOT NULL,
        "ram" real NOT NULL
    )
    """
    )

    # bench_browser
    await cur.execute(
        """
    CREATE TABLE "bench_browser" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 1,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "scaler_id" uuid,
        "scaler_bench_id" uuid,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "version" varchar,
        "target_version" varchar,
        "external_name" varchar,
        "external_id" varchar,
        "connection_uri" bytea,
        "debugger_uri" bytea,
        "view_uri" bytea,
        "client_id" uuid,
        "size" jsonb,
        "is_headless" boolean NOT NULL DEFAULT false,
        "is_insecure" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_file
    await cur.execute(
        """
    CREATE TABLE "bench_file" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "scaler_id" uuid,
        "scaler_bench_id" uuid,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "kind" smallint NOT NULL,
        "mime_type" varchar,
        "format" integer,
        "size" bigint NOT NULL,
        "sha256" varchar,
        "width" integer,
        "height" integer,
        "aspect_ratio" real,
        "codec" varchar,
        "duration" interval,
        "external_url" varchar,
        "inline_content" bytea,
        "retention" smallint NOT NULL,
        "expires_at" timestamp
    )
    """
    )

    # bench_stream
    await cur.execute(
        """
    CREATE TABLE "bench_stream" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "scaler_id" uuid,
        "scaler_bench_id" uuid,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp
    )
    """
    )

    # bench_secret
    await cur.execute(
        """
    CREATE TABLE "bench_secret" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "scaler_id" uuid,
        "scaler_bench_id" uuid,
        "status" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "value_type" jsonb NOT NULL,
        "value_packed" bytea
    )
    """
    )

    # bench_package
    await cur.execute(
        """
    CREATE TABLE "bench_package" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "slug" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "base_id" uuid,
        "main_channel_id" uuid
    )
    """
    )

    # bench_dependency
    await cur.execute(
        """
    CREATE TABLE "bench_dependency" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "depends_on_bench_id" uuid NOT NULL,
        "depends_on_packages_id" uuid[] NOT NULL,
        "depends_on_packages_bench_id" uuid[] NOT NULL
    )
    """
    )

    # bench_page
    await cur.execute(
        """
    CREATE TABLE "bench_page" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "parent_base_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[]
    )
    """
    )

    # bench_block
    await cur.execute(
        """
    CREATE TABLE "bench_block" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "line" jsonb,
        "node_id" uuid,
        "node_ck" uuid,
        "node_type" smallint,
        "node_bench_id" uuid
    )
    """
    )

    # bench_choice
    await cur.execute(
        """
    CREATE TABLE "bench_choice" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[]
    )
    """
    )

    # bench_class
    await cur.execute(
        """
    CREATE TABLE "bench_class" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[]
    )
    """
    )

    # bench_field
    await cur.execute(
        """
    CREATE TABLE "bench_field" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "icon" jsonb,
        "kind" smallint NOT NULL,
        "primitive_type" smallint,
        "bench_type" smallint,
        "base_type_id" uuid,
        "base_type_ck" uuid,
        "base_type_type" smallint,
        "base_type_bench_id" uuid,
        "base_field_types" smallint[],
        "property_field_types" smallint[],
        "oneof_id" uuid,
        "oneof_type" smallint,
        "default_packed" jsonb,
        "format" smallint,
        "condition" jsonb,
        "constraint" jsonb,
        "is_required" boolean NOT NULL DEFAULT false,
        "is_list" boolean NOT NULL DEFAULT false,
        "is_secret" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_option
    await cur.execute(
        """
    CREATE TABLE "bench_option" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "icon" jsonb
    )
    """
    )

    # bench_tag
    await cur.execute(
        """
    CREATE TABLE "bench_tag" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[]
    )
    """
    )

    # bench_flow
    await cur.execute(
        """
    CREATE TABLE "bench_flow" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "computed_values" jsonb[],
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "options" jsonb,
        "selection" jsonb,
        "roles_id" uuid[],
        "roles_bench_id" uuid[]
    )
    """
    )

    # bench_action
    await cur.execute(
        """
    CREATE TABLE "bench_action" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "computed_values" jsonb[],
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "category" smallint,
        "name" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "icon" jsonb,
        "text" jsonb,
        "options" jsonb,
        "selection" jsonb,
        "code" jsonb,
        "tool_id" uuid,
        "tool_type" smallint,
        "tool_bench_id" uuid,
        "position" jsonb
    )
    """
    )

    # bench_link
    await cur.execute(
        """
    CREATE TABLE "bench_link" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "computed_values" jsonb[],
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "source_id" uuid NOT NULL,
        "source_bench_id" uuid NOT NULL,
        "target_id" uuid NOT NULL,
        "target_bench_id" uuid NOT NULL,
        "options" jsonb,
        "trigger" smallint NOT NULL DEFAULT 1,
        "delay" interval,
        "is_manual" boolean NOT NULL DEFAULT false,
        "color" jsonb
    )
    """
    )

    # bench_trigger
    await cur.execute(
        """
    CREATE TABLE "bench_trigger" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "parent_base_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "text" jsonb,
        "effect" smallint NOT NULL,
        "scope_id" uuid,
        "scope_ck" uuid,
        "scope_type" smallint,
        "scope_bench_id" uuid,
        "run_root_id" uuid,
        "run_root_bench_id" uuid,
        "run_root_base_id" uuid,
        "run_id" uuid,
        "run_bench_id" uuid,
        "run_base_id" uuid,
        "interruption_id" uuid,
        "interruption_bench_id" uuid,
        "status" smallint NOT NULL DEFAULT 10,
        "processed_at" timestamp,
        "processed_count" integer NOT NULL DEFAULT 0,
        "processed_key" varchar,
        "closed_at" timestamp
    )
    """
    )

    # bench_kit
    await cur.execute(
        """
    CREATE TABLE "bench_kit" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "target_id" uuid,
        "target_ck" uuid,
        "target_type" smallint,
        "target_bench_id" uuid
    )
    """
    )

    # bench_view
    await cur.execute(
        """
    CREATE TABLE "bench_view" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" integer NOT NULL,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "subviews_packed" jsonb,
        "title" varchar,
        "value_type" jsonb,
        "node_id" uuid,
        "node_ck" uuid,
        "node_type" smallint,
        "node_bench_id" uuid,
        "node_base_id" uuid,
        "position" jsonb,
        "size" jsonb,
        "margin" jsonb,
        "padding" jsonb,
        "orientation" smallint,
        "alignment" smallint,
        "transform" jsonb,
        "constraint" jsonb,
        "selection" jsonb,
        "focus" jsonb,
        "is_hidden" boolean NOT NULL DEFAULT false,
        "is_disabled" boolean NOT NULL DEFAULT false,
        "is_input" boolean NOT NULL DEFAULT false,
        "is_inline" boolean NOT NULL DEFAULT false,
        "is_minimal" boolean NOT NULL DEFAULT false,
        "is_loading" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_database
    await cur.execute(
        """
    CREATE TABLE "bench_database" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[]
    )
    """
    )

    # bench_channel
    await cur.execute(
        """
    CREATE TABLE "bench_channel" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 1,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[]
    )
    """
    )

    # bench_role
    await cur.execute(
        """
    CREATE TABLE "bench_role" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "color" smallint
    )
    """
    )

    # bench_space
    await cur.execute(
        """
    CREATE TABLE "bench_space" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "name" varchar,
        "text" jsonb,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "focus" jsonb,
        "selection" jsonb,
        "inspection_id" uuid,
        "inspection_ck" uuid,
        "inspection_type" smallint,
        "inspection_bench_id" uuid,
        "inspection_base_id" uuid,
        "channel_id" uuid,
        "channel_bench_id" uuid,
        "thread_id" uuid,
        "thread_bench_id" uuid,
        "run_id" uuid,
        "run_bench_id" uuid,
        "run_base_id" uuid
    )
    """
    )

    # bench_thread
    await cur.execute(
        """
    CREATE TABLE "bench_thread" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 1,
        "channel_id" uuid NOT NULL,
        "scope_id" uuid,
        "scope_type" smallint,
        "scope_bench_id" uuid,
        "run_root_id" uuid,
        "run_root_base_id" uuid,
        "run_id" uuid,
        "run_base_id" uuid,
        "status" smallint NOT NULL DEFAULT 10,
        "closed_at" timestamp,
        "created_from_id" uuid,
        "title" varchar,
        "text" jsonb
    )
    """
    )

    # bench_message
    await cur.execute(
        """
    CREATE TABLE "bench_message" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 1,
        "platform" smallint NOT NULL DEFAULT 1,
        "channel_id" uuid NOT NULL,
        "thread_id" uuid,
        "scope_id" uuid,
        "scope_ck" uuid,
        "scope_type" smallint,
        "scope_bench_id" uuid,
        "run_root_id" uuid,
        "run_root_base_id" uuid,
        "run_id" uuid,
        "run_bench_id" uuid,
        "run_base_id" uuid,
        "status" smallint NOT NULL DEFAULT 30,
        "failed_at" timestamp,
        "sent_at" timestamp,
        "received_at" timestamp,
        "read_at" timestamp,
        "reply_to_id" uuid,
        "reply_to_bench_id" uuid,
        "forwarded_from_id" uuid,
        "forwarded_from_bench_id" uuid,
        "title" varchar,
        "text" jsonb,
        "value_packed" jsonb,
        "clazz_id" uuid,
        "clazz_bench_id" uuid,
        "nodes_id" uuid[],
        "nodes_ck" uuid[],
        "nodes_type" smallint[],
        "nodes_bench_id" uuid[],
        "nodes_base_id" uuid[],
        "selection" jsonb,
        "created_interruption_id" uuid,
        "created_interruption_bench_id" uuid,
        "created_run_id" uuid,
        "created_run_bench_id" uuid,
        "created_run_base_id" uuid,
        "created_thread_id" uuid
    )
    """
    )

    # bench_notification
    await cur.execute(
        """
    CREATE TABLE "bench_notification" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "channel_id" uuid,
        "thread_id" uuid,
        "status" smallint NOT NULL DEFAULT 30,
        "failed_at" timestamp,
        "sent_at" timestamp,
        "received_at" timestamp,
        "read_at" timestamp,
        "title" varchar,
        "text" jsonb,
        "nodes_id" uuid[],
        "nodes_ck" uuid[],
        "nodes_type" smallint[],
        "nodes_bench_id" uuid[],
        "nodes_base_id" uuid[],
        "message_id" uuid,
        "message_bench_id" uuid
    )
    """
    )

    # bench_membership
    await cur.execute(
        """
    CREATE TABLE "bench_membership" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "to_id" uuid NOT NULL,
        "to_type" smallint NOT NULL,
        "to_bench_id" uuid NOT NULL,
        "member_id" uuid NOT NULL
    )
    """
    )

    # bench_invite
    await cur.execute(
        """
    CREATE TABLE "bench_invite" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "bench_id" uuid,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "to_id" uuid NOT NULL,
        "to_type" smallint NOT NULL,
        "to_bench_id" uuid NOT NULL,
        "user_id" uuid,
        "user_email" varchar,
        "is_owner" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_session
    await cur.execute(
        """
    CREATE TABLE "bench_session" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "opened_at" timestamp,
        "closed_at" timestamp,
        "session_id" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_run
    await cur.execute(
        """
    CREATE TABLE "bench_run" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "parent_base_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "root_id" uuid,
        "root_base_id" uuid,
        "incoming_id" uuid[],
        "incoming_base_id" uuid[],
        "options" jsonb NOT NULL,
        "page_id" uuid,
        "channel_id" uuid,
        "thread_id" uuid,
        "status" smallint NOT NULL DEFAULT 1,
        "attempt" integer,
        "duration" interval,
        "scheduled_at" timestamp,
        "started_at" timestamp,
        "stopped_at" timestamp,
        "interrupted_at" timestamp,
        "paused_at" timestamp,
        "resumed_at" timestamp,
        "terminated_at" timestamp,
        "error" jsonb,
        "interruption_id" uuid,
        "flow_id" uuid,
        "kit_id" uuid,
        "action_id" uuid,
        "link_id" uuid,
        "plan_id" uuid,
        "plan_ck" uuid,
        "task_id" uuid,
        "task_ck" uuid,
        "trigger_id" uuid,
        "trigger_key" varchar,
        "resources_packed" jsonb,
        "inputs_packed" jsonb,
        "outputs_packed" jsonb,
        "name" varchar,
        "text" jsonb,
        "code" jsonb,
        "session_id" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_run_span
    await cur.execute(
        """
    CREATE TABLE "bench_run_span" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_base_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "root_id" uuid,
        "root_base_id" uuid,
        "severity" smallint NOT NULL DEFAULT 3,
        "status" smallint NOT NULL DEFAULT 2,
        "duration" interval,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "interrupted_at" timestamp,
        "interruption_id" uuid,
        "error" jsonb,
        "title" varchar,
        "text" jsonb,
        "code" jsonb,
        "nodes_id" uuid[],
        "nodes_ck" uuid[],
        "nodes_type" smallint[],
        "nodes_bench_id" uuid[],
        "nodes_base_id" uuid[],
        "session_id" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_interruption
    await cur.execute(
        """
    CREATE TABLE "bench_interruption" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_base_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "root_id" uuid,
        "root_base_id" uuid,
        "page_id" uuid,
        "page_bench_id" uuid,
        "flow_id" uuid,
        "flow_bench_id" uuid,
        "action_id" uuid,
        "action_bench_id" uuid,
        "link_id" uuid,
        "link_bench_id" uuid,
        "attempt" integer,
        "breakpoint_site" smallint,
        "status" smallint NOT NULL DEFAULT 10,
        "duration" interval,
        "closed_at" timestamp,
        "title" varchar,
        "text" jsonb,
        "inputs_packed" jsonb,
        "outputs_packed" jsonb,
        "response" smallint,
        "message_id" uuid,
        "task_id" uuid,
        "task_ck" uuid,
        "cancel_trigger_id" uuid,
        "cancel_trigger_bench_id" uuid,
        "complete_trigger_id" uuid,
        "complete_trigger_bench_id" uuid,
        "session_id" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_log
    await cur.execute(
        """
    CREATE TABLE "bench_log" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_type" smallint,
        "parent_base_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "severity" smallint NOT NULL,
        "title" varchar,
        "text" jsonb,
        "session_id" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_plan
    await cur.execute(
        """
    CREATE TABLE "bench_plan" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "parent_base_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "name" varchar,
        "on_terminate" smallint NOT NULL,
        "on_failure" smallint NOT NULL,
        "implemented_by_id" uuid,
        "implemented_by_base_id" uuid,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "error" jsonb,
        "session_id" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_task
    await cur.execute(
        """
    CREATE TABLE "bench_task" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "parent_base_id" uuid,
        "bench_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "template_at" timestamp,
        "owned_by_id" uuid,
        "owned_by_type" smallint,
        "mode" smallint NOT NULL DEFAULT 2,
        "subnode_packed" jsonb,
        "type" smallint NOT NULL DEFAULT 10,
        "name" varchar,
        "order_key" varchar,
        "icon" jsonb,
        "text" jsonb,
        "definition_id" uuid,
        "thread_id" uuid,
        "tags_id" uuid[],
        "implemented_by_id" uuid,
        "implemented_by_base_id" uuid,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "due_at" timestamp,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "error" jsonb,
        "clazz_id" uuid,
        "clazz_bench_id" uuid,
        "target_id" uuid,
        "target_type" smallint,
        "target_bench_id" uuid,
        "value_packed" jsonb,
        "interruption_id" uuid,
        "is_manual" boolean NOT NULL DEFAULT false,
        "session_id" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_scaler
    await cur.execute(
        'CREATE INDEX "bench_scaler_bench_idx_parent_id" ON "bench_scaler" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_store
    await cur.execute(
        'CREATE INDEX "bench_store_bench_idx_parent_id" ON "bench_store" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_machine
    await cur.execute(
        'CREATE INDEX "bench_machine_bench_idx_parent_id" ON "bench_machine" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_browser
    await cur.execute(
        'CREATE INDEX "bench_browser_bench_idx_parent_id" ON "bench_browser" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_file
    await cur.execute(
        'CREATE INDEX "bench_file_bench_idx_parent_id" ON "bench_file" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_stream
    await cur.execute(
        'CREATE INDEX "bench_stream_bench_idx_parent_id" ON "bench_stream" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_secret
    await cur.execute(
        'CREATE INDEX "bench_secret_bench_idx_parent_id" ON "bench_secret" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_package
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_package_bench_idx_bench_id_slug" ON "bench_package" USING BTREE (bench_id, slug)'
    )
    await cur.execute(
        'CREATE INDEX "bench_package_bench_idx_parent_id" ON "bench_package" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_dependency
    await cur.execute(
        'CREATE INDEX "bench_dependency_bench_idx_parent_id" ON "bench_dependency" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_page
    await cur.execute(
        'CREATE INDEX "bench_page_bench_idx_parent_id" ON "bench_page" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_block
    await cur.execute(
        'CREATE INDEX "bench_block_bench_idx_parent_id" ON "bench_block" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_choice
    await cur.execute(
        'CREATE INDEX "bench_choice_bench_idx_parent_id" ON "bench_choice" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_class
    await cur.execute(
        'CREATE INDEX "bench_class_bench_idx_parent_id" ON "bench_class" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_field
    await cur.execute(
        'CREATE INDEX "bench_field_bench_idx_parent_id" ON "bench_field" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_option
    await cur.execute(
        'CREATE INDEX "bench_option_bench_idx_parent_id" ON "bench_option" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_tag
    await cur.execute(
        'CREATE INDEX "bench_tag_bench_idx_parent_id" ON "bench_tag" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_flow
    await cur.execute(
        'CREATE INDEX "bench_flow_bench_idx_parent_id" ON "bench_flow" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_action
    await cur.execute(
        'CREATE INDEX "bench_action_bench_idx_parent_id" ON "bench_action" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_link
    await cur.execute(
        'CREATE INDEX "bench_link_bench_idx_parent_id" ON "bench_link" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_trigger
    await cur.execute(
        'CREATE INDEX "bench_trigger_bench_idx_parent_id" ON "bench_trigger" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_kit
    await cur.execute(
        'CREATE INDEX "bench_kit_bench_idx_parent_id" ON "bench_kit" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_view
    await cur.execute(
        'CREATE INDEX "bench_view_bench_idx_parent_id" ON "bench_view" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_database
    await cur.execute(
        'CREATE INDEX "bench_database_bench_idx_parent_id" ON "bench_database" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_channel
    await cur.execute(
        'CREATE INDEX "bench_channel_bench_idx_created_at" ON "bench_channel" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_channel_bench_idx_parent_id" ON "bench_channel" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_role
    await cur.execute(
        'CREATE INDEX "bench_role_bench_idx_parent_id" ON "bench_role" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_space
    await cur.execute(
        'CREATE INDEX "bench_space_bench_idx_parent_id" ON "bench_space" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_thread
    await cur.execute(
        'CREATE INDEX "bench_thread_bench_idx_created_at" ON "bench_thread" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_thread_bench_idx_parent_id" ON "bench_thread" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_message
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_created_at" ON "bench_message" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_parent_id" ON "bench_message" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_notification
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_created_at" ON "bench_notification" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_parent_id" ON "bench_notification" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_membership
    await cur.execute(
        'CREATE INDEX "bench_membership_bench_idx_parent_id" ON "bench_membership" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_invite
    await cur.execute(
        'CREATE INDEX "bench_invite_bench_idx_parent_id" ON "bench_invite" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_session
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_status" ON "bench_session" USING BTREE (status)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_created_at" ON "bench_session" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_parent_id" ON "bench_session" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_run
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_trigger_id_trigger_key" ON "bench_run" USING BTREE (trigger_id, trigger_key)'
    )
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_created_at" ON "bench_run" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_parent_id" ON "bench_run" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_run_span
    await cur.execute(
        'CREATE INDEX "bench_run_span_bench_idx_created_at" ON "bench_run_span" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_run_span_bench_idx_parent_id" ON "bench_run_span" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_interruption
    await cur.execute(
        'CREATE INDEX "bench_interruption_bench_idx_created_at" ON "bench_interruption" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_interruption_bench_idx_parent_id" ON "bench_interruption" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_log
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_created_at" ON "bench_log" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_parent_id" ON "bench_log" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_plan
    await cur.execute(
        'CREATE INDEX "bench_plan_bench_idx_created_at" ON "bench_plan" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_plan_bench_idx_parent_id" ON "bench_plan" USING BTREE (parent_id) INCLUDE (id)'
    )

    # bench_task
    await cur.execute(
        'CREATE INDEX "bench_task_bench_idx_created_at" ON "bench_task" USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_task_bench_idx_parent_id" ON "bench_task" USING BTREE (parent_id) INCLUDE (id)'
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "plpgsql"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pgcrypto"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')

    # bench_migration
    await cur.execute(
        """
    CREATE TABLE "bench_migration" (
        "id" integer NOT NULL PRIMARY KEY,
        "version" varchar NOT NULL,
        "has_global" boolean NOT NULL,
        "has_regional" boolean NOT NULL,
        "has_local" boolean NOT NULL,
        "applied_at" timestamp
    )
    """
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
