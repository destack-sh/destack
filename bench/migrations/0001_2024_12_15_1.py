# This migration was automatically generated on 2024.12.15. Edit as needed.
import psycopg

ID = 1
VERSION = "2024.12.15.1"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pgcrypto"')

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
        "main_handle_bench_id" uuid,
        "slug" varchar NOT NULL,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "owner_id" uuid,
        "owner_type" smallint,
        "region" smallint NOT NULL,
        "encryption_key" bytea NOT NULL,
        "policies" jsonb[] NOT NULL,
        "main_store_id" uuid,
        "main_server_id" uuid,
        "main_drive_id" uuid,
        "main_vault_id" uuid,
        "main_cache_id" uuid,
        "main_package_id" uuid,
        "main_package_ck" uuid
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
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "main_handle_bench_id" uuid,
        "slug" varchar,
        "name" varchar NOT NULL,
        "text" jsonb,
        "email" varchar,
        "icon" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL,
        "password_salt" bytea,
        "password_hash" bytea,
        "last_logged_in_at" timestamp,
        "is_staff" boolean NOT NULL DEFAULT false
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
        "slug" varchar NOT NULL
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
        "title" varchar NOT NULL,
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
        "space_ck" uuid,
        "space_bench_id" uuid,
        "machine_id" uuid,
        "machine_bench_id" uuid
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
        "created_by_ck" uuid,
        "created_by_type" smallint,
        "updated_at" timestamp NOT NULL,
        "updated_by_id" uuid,
        "updated_by_ck" uuid,
        "updated_by_type" smallint,
        "deleted_at" timestamp,
        "subnode_packed" jsonb,
        "main_handle_bench_id" uuid,
        "slug" varchar,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL
    )
    """
    )

    # bench_membership
    await cur.execute(
        """
    CREATE TABLE "bench_membership" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "user_id" uuid NOT NULL,
        "is_owner" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_invite
    await cur.execute(
        """
    CREATE TABLE "bench_invite" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "user_id" uuid,
        "user_email" varchar,
        "is_owner" boolean NOT NULL DEFAULT false,
        "roles_id" uuid[],
        "roles_ck" uuid[],
        "roles_bench_id" uuid[]
    )
    """
    )

    # bench_bench
    await cur.execute(
        'ALTER TABLE "bench_bench" ADD COLUMN "main_handle_id" uuid REFERENCES bench_handle ON DELETE SET NULL'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_bench_bench_idx_slug" ON bench_bench USING BTREE (slug)'
    )
    await cur.execute(
        'ALTER TABLE "bench_bench" ADD CONSTRAINT "bench_bench_bench_idx_slug" UNIQUE USING INDEX bench_bench_bench_idx_slug'
    )

    # bench_user
    await cur.execute(
        'ALTER TABLE "bench_user" ADD COLUMN "main_handle_id" uuid REFERENCES bench_handle ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_user" ADD COLUMN "main_bench_id" uuid REFERENCES bench_bench ON DELETE SET NULL'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_user_bench_idx_slug" ON bench_user USING BTREE (slug)'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_user_bench_idx_email" ON bench_user USING BTREE (email)'
    )
    await cur.execute(
        'ALTER TABLE "bench_user" ADD CONSTRAINT "bench_user_bench_idx_slug" UNIQUE USING INDEX bench_user_bench_idx_slug'
    )
    await cur.execute(
        'ALTER TABLE "bench_user" ADD CONSTRAINT "bench_user_bench_idx_email" UNIQUE USING INDEX bench_user_bench_idx_email'
    )

    # bench_handle
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_handle_bench_idx_slug" ON bench_handle USING BTREE (slug)'
    )
    await cur.execute(
        'ALTER TABLE "bench_handle" ADD CONSTRAINT "bench_handle_bench_idx_slug" UNIQUE USING INDEX bench_handle_bench_idx_slug'
    )

    # bench_client
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_client_bench_idx_access_token" ON bench_client USING BTREE (access_token)'
    )
    await cur.execute(
        'ALTER TABLE "bench_client" ADD CONSTRAINT "bench_client_bench_idx_access_token" UNIQUE USING INDEX bench_client_bench_idx_access_token'
    )

    # bench_organization
    await cur.execute(
        'ALTER TABLE "bench_organization" ADD COLUMN "main_handle_id" uuid REFERENCES bench_handle ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_organization" ADD COLUMN "main_bench_id" uuid REFERENCES bench_bench ON DELETE SET NULL'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_organization_bench_idx_slug" ON bench_organization USING BTREE (slug)'
    )
    await cur.execute(
        'ALTER TABLE "bench_organization" ADD CONSTRAINT "bench_organization_bench_idx_slug" UNIQUE USING INDEX bench_organization_bench_idx_slug'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pgcrypto"')

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

    # bench_server
    await cur.execute(
        """
    CREATE TABLE "bench_server" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "name" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "version" varchar NOT NULL,
        "target_version" varchar NOT NULL,
        "min_cpu" real,
        "max_cpu" real,
        "min_ram" real,
        "max_ram" real,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_store
    await cur.execute(
        """
    CREATE TABLE "bench_store" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "name" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
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
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_drive
    await cur.execute(
        """
    CREATE TABLE "bench_drive" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "name" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_vault
    await cur.execute(
        """
    CREATE TABLE "bench_vault" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "name" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_cache
    await cur.execute(
        """
    CREATE TABLE "bench_cache" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "name" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 1
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
        "type" smallint NOT NULL DEFAULT 1,
        "title" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "owned_by_id" uuid,
        "owned_by_ck" uuid,
        "owned_by_type" smallint,
        "owned_by_bench_id" uuid,
        "owned_by_base_ck" uuid,
        "owned_by_base_bench_id" uuid,
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
        "target_cpu" real,
        "ram" real NOT NULL,
        "target_ram" real,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_browser
    await cur.execute(
        """
    CREATE TABLE "bench_browser" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "type" smallint NOT NULL DEFAULT 1,
        "title" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "owned_by_id" uuid,
        "owned_by_ck" uuid,
        "owned_by_type" smallint,
        "owned_by_bench_id" uuid,
        "owned_by_base_ck" uuid,
        "owned_by_base_bench_id" uuid,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "version" varchar,
        "target_version" varchar,
        "external_name" varchar,
        "mode" smallint NOT NULL DEFAULT 1
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
        "title" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "owned_by_id" uuid,
        "owned_by_ck" uuid,
        "owned_by_type" smallint,
        "owned_by_bench_id" uuid,
        "owned_by_base_ck" uuid,
        "owned_by_base_bench_id" uuid,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "kind" smallint NOT NULL,
        "external_url" varchar,
        "inline_content" bytea,
        "type" smallint NOT NULL,
        "mime_type" varchar,
        "format" integer,
        "size" bigint NOT NULL,
        "sha256" varchar,
        "width" integer,
        "height" integer,
        "aspect_ratio" real,
        "codec" varchar,
        "duration" interval,
        "bitrate" integer,
        "channels" integer,
        "sample_rate" integer,
        "retention" smallint NOT NULL,
        "expires_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 1
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
        "title" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "owned_by_id" uuid,
        "owned_by_ck" uuid,
        "owned_by_type" smallint,
        "owned_by_bench_id" uuid,
        "owned_by_base_ck" uuid,
        "owned_by_base_bench_id" uuid,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "mode" smallint NOT NULL DEFAULT 1
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
        "title" varchar NOT NULL,
        "status" smallint NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "occupancy" smallint NOT NULL,
        "owned_by_id" uuid,
        "owned_by_ck" uuid,
        "owned_by_type" smallint,
        "owned_by_bench_id" uuid,
        "owned_by_base_ck" uuid,
        "owned_by_base_bench_id" uuid,
        "activated_at" timestamp,
        "deactivated_at" timestamp,
        "reset_at" timestamp,
        "suspended_at" timestamp,
        "decommissioned_at" timestamp,
        "active_at" timestamp,
        "value_type" jsonb NOT NULL,
        "value_packed" bytea,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pgcrypto"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pg_trgm"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "timescaledb"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "plpgsql"')

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

    # bench_package
    await cur.execute(
        """
    CREATE TABLE "bench_package" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "bench_id" uuid NOT NULL,
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
        "slug" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "owned_by_id" uuid,
        "owned_by_ck" uuid,
        "owned_by_type" smallint,
        "owned_by_bench_id" uuid,
        "owned_by_base_ck" uuid,
        "owned_by_base_bench_id" uuid,
        "base_id" uuid,
        "base_ck" uuid
    )
    """
    )

    # bench_dependency
    await cur.execute(
        """
    CREATE TABLE "bench_dependency" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
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
        "depends_on_bench_id" uuid NOT NULL,
        "depends_on_packages_id" uuid[] NOT NULL,
        "depends_on_packages_ck" uuid[] NOT NULL,
        "depends_on_packages_bench_id" uuid[] NOT NULL,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_space
    await cur.execute(
        """
    CREATE TABLE "bench_space" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
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
        "text" jsonb,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "policies" jsonb[],
        "owned_by_id" uuid,
        "owned_by_ck" uuid,
        "owned_by_type" smallint,
        "owned_by_bench_id" uuid,
        "owned_by_base_ck" uuid,
        "owned_by_base_bench_id" uuid,
        "focus" jsonb,
        "selection" jsonb,
        "inspection_id" uuid,
        "inspection_ck" uuid,
        "inspection_type" smallint,
        "inspection_bench_id" uuid,
        "inspection_base_ck" uuid,
        "inspection_base_bench_id" uuid,
        "base_id" uuid,
        "base_ck" uuid,
        "base_type" smallint,
        "base_bench_id" uuid,
        "base_base_ck" uuid,
        "base_base_bench_id" uuid,
        "run_id" uuid,
        "run_bench_id" uuid,
        "run_base_ck" uuid,
        "run_base_bench_id" uuid,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_block
    await cur.execute(
        """
    CREATE TABLE "bench_block" (
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
        "variables_packed" jsonb,
        "run_options" jsonb,
        "policies" jsonb[],
        "delegated_policies" jsonb[],
        "roles_id" uuid[],
        "roles_ck" uuid[],
        "roles_bench_id" uuid[],
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

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
        "is_paused" boolean NOT NULL DEFAULT false,
        "processed_epoch" integer,
        "schedule" jsonb,
        "message_id" uuid,
        "message_ck" uuid,
        "message_bench_id" uuid,
        "condition" jsonb,
        "mode" smallint NOT NULL DEFAULT 1
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
        "package_id" uuid NOT NULL,
        "package_ck" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "template_base_ck" uuid,
        "template_base_bench_id" uuid,
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
        "type" smallint NOT NULL DEFAULT 1,
        "name" varchar NOT NULL,
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
        "base_field_type" smallint,
        "oneof_id" uuid,
        "oneof_ck" uuid,
        "oneof_type" smallint,
        "oneof_base_ck" uuid,
        "partial_scope" smallint,
        "default_packed" jsonb,
        "format" smallint,
        "condition" jsonb,
        "constraint" jsonb,
        "is_required" boolean NOT NULL DEFAULT false,
        "is_list" boolean NOT NULL DEFAULT false,
        "is_secret" boolean NOT NULL DEFAULT false,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_query
    await cur.execute(
        """
    CREATE TABLE "bench_query" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid,
        "parent_ck" uuid,
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
        "node_type" smallint NOT NULL,
        "base_block_id" uuid,
        "base_block_ck" uuid,
        "base_block_bench_id" uuid,
        "roots_id" uuid[] NOT NULL,
        "roots_ck" uuid[] NOT NULL,
        "roots_type" smallint[] NOT NULL,
        "roots_bench_id" uuid[] NOT NULL,
        "roots_base_ck" uuid[],
        "roots_base_bench_id" uuid[],
        "filter" jsonb,
        "sort" jsonb[],
        "aggregation" jsonb,
        "ancestor_types" smallint[] NOT NULL,
        "descendant_types" smallint[] NOT NULL,
        "select" jsonb,
        "include_deleted" boolean NOT NULL DEFAULT false,
        "first" integer,
        "skip" integer,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_view
    await cur.execute(
        """
    CREATE TABLE "bench_view" (
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
        "title" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "icon" jsonb,
        "subviews_packed" jsonb,
        "value_type" jsonb,
        "node_id" uuid,
        "node_ck" uuid,
        "node_type" smallint,
        "node_bench_id" uuid,
        "node_base_ck" uuid,
        "node_base_bench_id" uuid,
        "variant" smallint,
        "font" jsonb,
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
        "is_loading" boolean NOT NULL DEFAULT false,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_step
    await cur.execute(
        """
    CREATE TABLE "bench_step" (
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
        "position" jsonb,
        "size" jsonb,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_pipe
    await cur.execute(
        """
    CREATE TABLE "bench_pipe" (
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
        "text" jsonb,
        "source_id" uuid NOT NULL,
        "source_ck" uuid NOT NULL,
        "source_bench_id" uuid NOT NULL,
        "target_id" uuid NOT NULL,
        "target_ck" uuid NOT NULL,
        "target_bench_id" uuid NOT NULL,
        "run_options" jsonb,
        "condition" jsonb,
        "constraint" jsonb,
        "mapping" jsonb,
        "delay" interval,
        "mode" smallint NOT NULL DEFAULT 1,
        "color" jsonb,
        "is_hidden" boolean NOT NULL DEFAULT false,
        "is_name_shown" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_badge
    await cur.execute(
        """
    CREATE TABLE "bench_badge" (
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
        "name" varchar NOT NULL,
        "delegated_policies" jsonb[] NOT NULL,
        "expires_at" timestamp,
        "key" bytea,
        "key_hash" bytea,
        "password" bytea,
        "password_hash" bytea,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_message
    await cur.execute(
        """
    CREATE TABLE "bench_message" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "parent_base_ck" uuid,
        "package_id" uuid NOT NULL,
        "package_ck" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
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
        "type" smallint NOT NULL DEFAULT 1,
        "status" smallint NOT NULL DEFAULT 4,
        "origin_id" uuid,
        "origin_ck" uuid,
        "origin_type" smallint,
        "origin_bench_id" uuid,
        "origin_base_ck" uuid,
        "origin_base_bench_id" uuid,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "reply_to_id" uuid,
        "reply_to_bench_id" uuid,
        "reply_to_base_ck" uuid,
        "reply_to_base_bench_id" uuid,
        "title" varchar,
        "text" jsonb,
        "value_packed" jsonb,
        "expires_at" timestamp,
        "read_at" timestamp,
        "is_pinned" boolean NOT NULL DEFAULT false,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_session
    await cur.execute(
        """
    CREATE TABLE "bench_session" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_ck" uuid,
        "package_id" uuid NOT NULL,
        "package_ck" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
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
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "opened_at" timestamp,
        "closed_at" timestamp,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid,
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_run
    await cur.execute(
        """
    CREATE TABLE "bench_run" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_ck" uuid,
        "parent_type" smallint,
        "parent_base_ck" uuid,
        "package_id" uuid NOT NULL,
        "package_ck" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
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
        "root_id" uuid,
        "root_base_ck" uuid,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "pipe_id" uuid,
        "pipe_ck" uuid,
        "pipe_bench_id" uuid,
        "incoming_id" uuid[],
        "incoming_base_ck" uuid[],
        "outgoing_id" uuid[],
        "outgoing_base_ck" uuid[],
        "context" jsonb,
        "options" jsonb NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "attempts" jsonb[] NOT NULL,
        "error" jsonb,
        "scheduled_at" timestamp,
        "scheduled_epoch" integer,
        "started_at" timestamp,
        "started_epoch" integer,
        "stopped_at" timestamp,
        "interrupted_at" timestamp,
        "interrupt_id" uuid,
        "interrupt_ck" uuid,
        "paused_at" timestamp,
        "resumed_at" timestamp,
        "terminated_at" timestamp,
        "terminated_epoch" integer,
        "variables_packed" jsonb,
        "inputs_packed" jsonb,
        "intermediates_packed" jsonb,
        "outputs_packed" jsonb,
        "logs" jsonb[] NOT NULL,
        "spans" jsonb[] NOT NULL,
        "events" jsonb[] NOT NULL,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid,
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_interrupt
    await cur.execute(
        """
    CREATE TABLE "bench_interrupt" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_base_ck" uuid,
        "package_id" uuid NOT NULL,
        "package_ck" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
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
        "root_id" uuid,
        "root_base_ck" uuid,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "pipe_id" uuid,
        "pipe_ck" uuid,
        "pipe_bench_id" uuid,
        "attempt_no" integer,
        "breakpoint_site" smallint,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" interval,
        "closed_at" timestamp,
        "trigger_id" uuid,
        "trigger_ck" uuid,
        "trigger_bench_id" uuid,
        "inputs_packed" jsonb,
        "outputs_packed" jsonb,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid,
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_log
    await cur.execute(
        """
    CREATE TABLE "bench_log" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid,
        "parent_ck" uuid,
        "package_id" uuid NOT NULL,
        "package_ck" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
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
        "kind" smallint NOT NULL,
        "level" smallint NOT NULL DEFAULT 3,
        "change_id" uuid,
        "undo_of_id" uuid,
        "type" smallint,
        "node_id" uuid,
        "node_ck" uuid,
        "node_type" smallint,
        "node_base_ck" uuid,
        "node_data" jsonb,
        "operations" jsonb[] NOT NULL,
        "category" smallint,
        "vignette" jsonb,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid,
        "identity_id" uuid,
        "identity_ck" uuid,
        "identity_bench_id" uuid,
        "mode" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_package
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_package_bench_idx_bench_id_slug" ON bench_package USING BTREE (bench_id, slug)'
    )
    await cur.execute(
        'ALTER TABLE "bench_package" ADD CONSTRAINT "bench_package_bench_idx_bench_id_slug" UNIQUE USING INDEX bench_package_bench_idx_bench_id_slug'
    )

    # bench_badge
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_badge_bench_idx_key" ON bench_badge USING BTREE (key)'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_badge_bench_idx_key_hash" ON bench_badge USING BTREE (key_hash)'
    )
    await cur.execute(
        'ALTER TABLE "bench_badge" ADD CONSTRAINT "bench_badge_bench_idx_key" UNIQUE USING INDEX bench_badge_bench_idx_key'
    )
    await cur.execute(
        'ALTER TABLE "bench_badge" ADD CONSTRAINT "bench_badge_bench_idx_key_hash" UNIQUE USING INDEX bench_badge_bench_idx_key_hash'
    )

    # bench_message
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_created_at" ON bench_message USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_created_epoch" ON bench_message USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_package_id_created_at" ON bench_message USING BTREE (package_id, created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_message_bench_idx_package_id_created_epoch" ON bench_message USING BTREE (package_id, created_epoch)'
    )

    # bench_session
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_status" ON bench_session USING BTREE (status)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_created_at" ON bench_session USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_created_epoch" ON bench_session USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_package_id_created_at" ON bench_session USING BTREE (package_id, created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_package_id_created_epoch" ON bench_session USING BTREE (package_id, created_epoch)'
    )

    # bench_run
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_created_at" ON bench_run USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_created_epoch" ON bench_run USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_package_id_created_at" ON bench_run USING BTREE (package_id, created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_run_bench_idx_package_id_created_epoch" ON bench_run USING BTREE (package_id, created_epoch)'
    )

    # bench_interrupt
    await cur.execute(
        'CREATE INDEX "bench_interrupt_bench_idx_created_at" ON bench_interrupt USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_interrupt_bench_idx_created_epoch" ON bench_interrupt USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_interrupt_bench_idx_package_id_created_at" ON bench_interrupt USING BTREE (package_id, created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_interrupt_bench_idx_package_id_created_epoch" ON bench_interrupt USING BTREE (package_id, created_epoch)'
    )

    # bench_log
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_created_at" ON bench_log USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_created_epoch" ON bench_log USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_package_id_created_at" ON bench_log USING BTREE (package_id, created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_log_bench_idx_package_id_created_epoch" ON bench_log USING BTREE (package_id, created_epoch)'
    )


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
