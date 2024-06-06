# This migration was automatically generated on 2024.06.06. Edit as needed.
import psycopg

ID = 1
VERSION = "2024.06.06.1"
HAS_GLOBAL = True
HAS_LOCAL = True


#
# Global DB for core Bench nodes (runs once)
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
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "main_handle_bench_id" uuid,
        "slug" varchar NOT NULL,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "owner_id" uuid,
        "owner_type" smallint,
        "region" smallint NOT NULL,
        "encryption_key" bytea NOT NULL,
        "policies" jsonb[] NOT NULL
    )
    """
    )

    # bench_environment
    await cur.execute(
        """
    CREATE TABLE "bench_environment" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "policies" jsonb[] NOT NULL
    )
    """
    )

    # bench_branch
    await cur.execute(
        """
    CREATE TABLE "bench_branch" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "name" varchar NOT NULL,
        "slug" varchar,
        "text" jsonb,
        "icon" jsonb,
        "policies" jsonb[] NOT NULL,
        "is_overlay" boolean NOT NULL DEFAULT false,
        "is_light" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_package
    await cur.execute(
        """
    CREATE TABLE "bench_package" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "slug" varchar,
        "text" jsonb,
        "icon" jsonb,
        "policies" jsonb[] NOT NULL,
        "is_snapshot" boolean NOT NULL DEFAULT false,
        "is_overlay" boolean NOT NULL DEFAULT false,
        "is_paused" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_dependency
    await cur.execute(
        """
    CREATE TABLE "bench_dependency" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "scopes_id" uuid[] NOT NULL,
        "scopes_ck" uuid[] NOT NULL,
        "scopes_bench_id" uuid[] NOT NULL,
        "dependency_id" uuid NOT NULL,
        "dependency_bench_id" uuid NOT NULL,
        "dependency_scopes_id" uuid[] NOT NULL,
        "dependency_scopes_ck" uuid[] NOT NULL,
        "dependency_scopes_bench_id" uuid[] NOT NULL
    )
    """
    )

    # bench_upgrade
    await cur.execute(
        """
    CREATE TABLE "bench_upgrade" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "name" varchar NOT NULL,
        "title" varchar,
        "text" jsonb
    )
    """
    )

    # bench_space
    await cur.execute(
        """
    CREATE TABLE "bench_space" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "text" jsonb,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "policies" jsonb[],
        "bar_position" smallint DEFAULT 1,
        "focus" jsonb,
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
        "base_base_bench_id" uuid
    )
    """
    )

    # bench_link
    await cur.execute(
        """
    CREATE TABLE "bench_link" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "reference_id" uuid,
        "reference_ck" uuid,
        "reference_type" smallint,
        "reference_bench_id" uuid,
        "reference_base_ck" uuid,
        "reference_base_bench_id" uuid,
        "order_key" varchar
    )
    """
    )

    # bench_notice
    await cur.execute(
        """
    CREATE TABLE "bench_notice" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "parent_base_ck" uuid,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "kind" smallint NOT NULL,
        "type" smallint NOT NULL,
        "subject_id" uuid,
        "subject_ck" uuid,
        "subject_type" smallint,
        "subject_bench_id" uuid,
        "subject_base_ck" uuid,
        "subject_base_bench_id" uuid,
        "path" jsonb,
        "properties_ptr" jsonb[],
        "title" varchar,
        "text" jsonb
    )
    """
    )

    # bench_block
    await cur.execute(
        """
    CREATE TABLE "bench_block" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "policies" jsonb[] NOT NULL,
        "bases_id" uuid[],
        "bases_ck" uuid[],
        "bases_bench_id" uuid[],
        "builtin_base" jsonb,
        "text" jsonb,
        "icon" jsonb,
        "visibility" smallint,
        "value_packed" jsonb,
        "secret_value_packed" bytea,
        "code" jsonb,
        "run" jsonb,
        "delegated_policies" jsonb[] NOT NULL,
        "is_builtin" boolean NOT NULL DEFAULT false,
        "is_page" boolean NOT NULL DEFAULT false,
        "is_protocol" boolean NOT NULL DEFAULT false,
        "is_template" boolean NOT NULL DEFAULT false,
        "is_materialized" boolean NOT NULL DEFAULT false,
        "is_paused" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_trigger
    await cur.execute(
        """
    CREATE TABLE "bench_trigger" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "processed_epoch" integer,
        "schedule" jsonb,
        "signal_id" uuid,
        "signal_ck" uuid,
        "signal_bench_id" uuid,
        "condition" jsonb,
        "is_paused" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_field
    await cur.execute(
        """
    CREATE TABLE "bench_field" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "template_base_ck" uuid,
        "template_base_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "name" varchar NOT NULL,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "zone" smallint NOT NULL DEFAULT 1,
        "text" jsonb,
        "icon" jsonb,
        "value_packed" jsonb,
        "kind" smallint,
        "primitive_type" smallint,
        "bench_type" smallint,
        "base_type_id" uuid,
        "base_type_ck" uuid,
        "base_type_type" smallint,
        "base_type_bench_id" uuid,
        "base_field_zone" smallint,
        "default_packed" jsonb,
        "visibility" smallint,
        "format_hint" smallint,
        "condition" jsonb,
        "constraint" jsonb,
        "is_list" boolean NOT NULL DEFAULT false,
        "is_secret" boolean NOT NULL DEFAULT false,
        "is_required" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_query
    await cur.execute(
        """
    CREATE TABLE "bench_query" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "name" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "node_type" smallint NOT NULL,
        "base_id" uuid,
        "base_ck" uuid,
        "base_bench_id" uuid,
        "filter" jsonb,
        "sort" jsonb[]
    )
    """
    )

    # bench_view
    await cur.execute(
        """
    CREATE TABLE "bench_view" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "title" varchar,
        "text" jsonb,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "icon" jsonb,
        "value_type" jsonb,
        "value_packed" jsonb,
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
        "selection" jsonb,
        "focus" jsonb,
        "expansion" jsonb,
        "is_visible" boolean DEFAULT true,
        "is_disabled" boolean DEFAULT false,
        "is_input" boolean DEFAULT false,
        "is_inline" boolean DEFAULT false,
        "is_template" boolean DEFAULT false,
        "is_loading" boolean DEFAULT false
    )
    """
    )

    # bench_step
    await cur.execute(
        """
    CREATE TABLE "bench_step" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "type" smallint NOT NULL DEFAULT 1,
        "name" varchar,
        "order_key" varchar NOT NULL DEFAULT 'a0'::character varying,
        "text" jsonb,
        "code" jsonb,
        "run" jsonb,
        "connections" jsonb[] NOT NULL,
        "value_type" jsonb,
        "value_packed" jsonb,
        "secret_value_packed" bytea,
        "node_id" uuid,
        "node_ck" uuid,
        "node_type" smallint,
        "node_bench_id" uuid,
        "condition" jsonb,
        "is_template" boolean NOT NULL DEFAULT false
    )
    """
    )

    # bench_badge
    await cur.execute(
        """
    CREATE TABLE "bench_badge" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "name" varchar NOT NULL,
        "delegated_policies" jsonb[] NOT NULL,
        "expires_at" timestamp,
        "key" bytea,
        "key_hash" bytea,
        "password" bytea,
        "password_hash" bytea
    )
    """
    )

    # bench_role
    await cur.execute(
        """
    CREATE TABLE "bench_role" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "type_id" uuid NOT NULL,
        "type_ck" uuid NOT NULL,
        "type_bench_id" uuid NOT NULL
    )
    """
    )

    # bench_identity
    await cur.execute(
        """
    CREATE TABLE "bench_identity" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "type_id" uuid NOT NULL,
        "type_ck" uuid NOT NULL,
        "type_bench_id" uuid NOT NULL
    )
    """
    )

    # bench_membership
    await cur.execute(
        """
    CREATE TABLE "bench_membership" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
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
        "ck" uuid NOT NULL,
        "parent_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "template_id" uuid,
        "template_ck" uuid,
        "template_bench_id" uuid,
        "templated_epoch" bigint,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "set_properties" integer[] NOT NULL,
        "user_id" uuid,
        "user_email" varchar,
        "is_owner" boolean NOT NULL DEFAULT false,
        "roles_id" uuid[],
        "roles_ck" uuid[],
        "roles_bench_id" uuid[]
    )
    """
    )

    # bench_server
    await cur.execute(
        """
    CREATE TABLE "bench_server" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "profile" smallint NOT NULL,
        "current_profile" smallint,
        "version" varchar,
        "current_version" varchar,
        "active_at" timestamp,
        "bumped_at" timestamp
    )
    """
    )

    # bench_store
    await cur.execute(
        """
    CREATE TABLE "bench_store" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "version" varchar,
        "current_version" varchar,
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
        "parent_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "profile" smallint NOT NULL,
        "current_profile" smallint,
        "version" varchar,
        "current_version" varchar,
        "external_name" varchar,
        "external_id" varchar,
        "connection_uri" bytea,
        "started_at" timestamp,
        "terminated_at" timestamp,
        "active_at" timestamp
    )
    """
    )

    # bench_drive
    await cur.execute(
        """
    CREATE TABLE "bench_drive" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL DEFAULT 1
    )
    """
    )

    # bench_blob
    await cur.execute(
        """
    CREATE TABLE "bench_blob" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "name" varchar NOT NULL,
        "text" jsonb,
        "region" smallint NOT NULL,
        "status" smallint NOT NULL DEFAULT 1,
        "sha512" varchar NOT NULL,
        "size" bigint NOT NULL,
        "mime_type" varchar NOT NULL,
        "retention" smallint NOT NULL,
        "expires_at" timestamp
    )
    """
    )

    # bench_handle
    await cur.execute(
        """
    CREATE TABLE "bench_handle" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "bench_id" uuid,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "slug" varchar NOT NULL
    )
    """
    )

    # bench_user
    await cur.execute(
        """
    CREATE TABLE "bench_user" (
        "id" uuid NOT NULL PRIMARY KEY,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "updated_at" timestamp NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "main_handle_bench_id" uuid,
        "slug" varchar,
        "name" varchar NOT NULL,
        "text" jsonb,
        "email" varchar NOT NULL,
        "icon" jsonb,
        "status" smallint NOT NULL,
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
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "updated_at" timestamp NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "main_handle_bench_id" uuid,
        "slug" varchar,
        "name" varchar NOT NULL,
        "text" jsonb,
        "icon" jsonb,
        "status" smallint NOT NULL
    )
    """
    )

    # bench_client
    await cur.execute(
        """
    CREATE TABLE "bench_client" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "bench_id" uuid,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "type" smallint NOT NULL,
        "name" varchar NOT NULL,
        "device_type" varchar,
        "device_name" varchar,
        "operating_system" varchar,
        "browser_name" varchar,
        "browser_version" varchar,
        "place_id" varchar,
        "access_token" varchar,
        "seen_at" timestamp NOT NULL,
        "logged_in_at" timestamp,
        "space_id" uuid,
        "space_ck" uuid,
        "space_bench_id" uuid,
        "machine_bench_id" uuid
    )
    """
    )

    # bench_bench
    await cur.execute(
        'ALTER TABLE "bench_bench" ADD COLUMN "main_handle_id" uuid REFERENCES bench_handle ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_bench" ADD COLUMN "main_environment_id" uuid REFERENCES bench_environment ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_bench" ADD COLUMN "main_branch_id" uuid REFERENCES bench_branch ON DELETE SET NULL'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_bench_bench_idx_slug" ON bench_bench USING BTREE (slug)'
    )
    await cur.execute(
        'ALTER TABLE "bench_bench" ADD CONSTRAINT "bench_bench_bench_idx_slug" UNIQUE USING INDEX bench_bench_bench_idx_slug'
    )

    # bench_environment
    await cur.execute(
        'ALTER TABLE "bench_environment" ADD COLUMN "server_id" uuid NOT NULL REFERENCES bench_server ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_environment" ADD COLUMN "store_id" uuid NOT NULL REFERENCES bench_store ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_environment" ADD COLUMN "drive_id" uuid NOT NULL REFERENCES bench_drive ON DELETE SET NULL'
    )

    # bench_branch
    await cur.execute(
        'ALTER TABLE "bench_branch" ADD COLUMN "main_package_id" uuid REFERENCES bench_package ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_branch" ADD COLUMN "base_branch_id" uuid REFERENCES bench_branch ON DELETE SET NULL'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_branch_bench_idx_bench_id_slug" ON bench_branch USING BTREE (bench_id, slug)'
    )
    await cur.execute(
        'ALTER TABLE "bench_branch" ADD CONSTRAINT "bench_branch_bench_idx_bench_id_slug" UNIQUE USING INDEX bench_branch_bench_idx_bench_id_slug'
    )

    # bench_package
    await cur.execute(
        'ALTER TABLE "bench_package" ADD COLUMN "environment_id" uuid NOT NULL REFERENCES bench_environment ON DELETE SET NULL'
    )
    await cur.execute(
        'ALTER TABLE "bench_package" ADD COLUMN "base_package_id" uuid REFERENCES bench_package ON DELETE SET NULL'
    )
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

    # bench_blob
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_blob_bench_idx_parent_id_sha512" ON bench_blob USING BTREE (parent_id, sha512)'
    )
    await cur.execute(
        'ALTER TABLE "bench_blob" ADD CONSTRAINT "bench_blob_bench_idx_parent_id_sha512" UNIQUE USING INDEX bench_blob_bench_idx_parent_id_sha512'
    )

    # bench_handle
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_handle_bench_idx_slug" ON bench_handle USING BTREE (slug)'
    )
    await cur.execute(
        'ALTER TABLE "bench_handle" ADD CONSTRAINT "bench_handle_bench_slug_is_slug" CHECK (((slug)::text ~ \'^[a-z0-9-]{3,}$\'::text))'
    )
    await cur.execute(
        'ALTER TABLE "bench_handle" ADD CONSTRAINT "bench_handle_bench_idx_slug" UNIQUE USING INDEX bench_handle_bench_idx_slug'
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

    # bench_client
    await cur.execute(
        'ALTER TABLE "bench_client" ADD COLUMN "machine_id" uuid REFERENCES bench_machine ON DELETE SET NULL'
    )
    await cur.execute(
        'CREATE UNIQUE INDEX "bench_client_bench_idx_access_token" ON bench_client USING BTREE (access_token)'
    )
    await cur.execute(
        'ALTER TABLE "bench_client" ADD CONSTRAINT "bench_client_bench_idx_access_token" UNIQUE USING INDEX bench_client_bench_idx_access_token'
    )


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB for Bench-local nodes (records, runs, signals, etc.) (runs for every Bench)
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "uuid-ossp"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "timescaledb"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pg_trgm"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "plpgsql"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "pgcrypto"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "bloom"')
    await cur.execute('CREATE EXTENSION IF NOT EXISTS "vector"')

    # bench_migration
    await cur.execute(
        """
    CREATE TABLE "bench_migration" (
        "id" integer NOT NULL PRIMARY KEY,
        "version" varchar NOT NULL,
        "has_global" boolean NOT NULL,
        "has_local" boolean NOT NULL,
        "applied_at" timestamp
    )
    """
    )

    # bench_record_shared
    await cur.execute(
        """
    CREATE TABLE "bench_record_shared" (
        "id" uuid NOT NULL PRIMARY KEY,
        "ck" uuid NOT NULL,
        "revision" bigint NOT NULL DEFAULT 0,
        "created_at" timestamp NOT NULL DEFAULT now(),
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL DEFAULT now(),
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "block_tk" uuid NOT NULL,
        "block_ck" uuid NOT NULL,
        "block_id" uuid NOT NULL,
        "value_packed" jsonb,
        "secret_value_packed" bytea
    )
    """
    )

    # bench_session
    await cur.execute(
        """
    CREATE TABLE "bench_session" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" real,
        "opened_at" timestamp,
        "closed_at" timestamp,
        "client_id" uuid,
        "client_bench_id" uuid,
        "server_id" uuid,
        "server_bench_id" uuid,
        "machine_id" uuid,
        "machine_bench_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_run
    await cur.execute(
        """
    CREATE TABLE "bench_run" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "parent_base_ck" uuid,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "kind" smallint NOT NULL,
        "root_id" uuid NOT NULL,
        "root_base_ck" uuid,
        "code" jsonb,
        "text" jsonb,
        "options" jsonb,
        "status" smallint NOT NULL DEFAULT 1,
        "duration" real,
        "scheduled_at" timestamp,
        "scheduled_epoch" integer,
        "started_at" timestamp,
        "started_epoch" integer,
        "paused_at" timestamp,
        "terminated_at" timestamp,
        "terminated_epoch" integer,
        "attempts" jsonb[] NOT NULL,
        "inputs_packed" jsonb,
        "inputs_secret_packed" bytea,
        "outputs_packed" jsonb,
        "outputs_secret_packed" bytea,
        "value_packed" jsonb,
        "value_secret_packed" bytea,
        "error" jsonb,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_signal
    await cur.execute(
        """
    CREATE TABLE "bench_signal" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "type_id" uuid,
        "type_ck" uuid,
        "type_bench_id" uuid,
        "value_packed" jsonb,
        "secret_value_packed" bytea,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_log
    await cur.execute(
        """
    CREATE TABLE "bench_log" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "kind" smallint NOT NULL,
        "level" smallint NOT NULL DEFAULT 3,
        "type" smallint,
        "node_id" uuid,
        "node_ck" uuid,
        "node_type" smallint,
        "node_base_ck" uuid,
        "properties" smallint[] NOT NULL,
        "old_node_packed" jsonb,
        "old_node_secret_packed" bytea,
        "new_node_packed" jsonb,
        "new_node_secret_packed" bytea,
        "new_revision" bigint,
        "title" varchar,
        "text" jsonb,
        "value_packed" jsonb,
        "secret_value_packed" bytea,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_notification
    await cur.execute(
        """
    CREATE TABLE "bench_notification" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "kind" smallint NOT NULL,
        "type_id" uuid,
        "type_ck" uuid,
        "type_bench_id" uuid,
        "expires_at" timestamp,
        "read_at" timestamp,
        "title" varchar,
        "text" jsonb,
        "value_packed" jsonb,
        "secret_value_packed" bytea,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_message
    await cur.execute(
        """
    CREATE TABLE "bench_message" (
        "id" uuid NOT NULL PRIMARY KEY,
        "parent_id" uuid NOT NULL,
        "parent_ck" uuid NOT NULL,
        "parent_type" smallint NOT NULL,
        "parent_base_ck" uuid,
        "package_id" uuid NOT NULL,
        "bench_id" uuid NOT NULL,
        "revision" bigint NOT NULL,
        "created_at" timestamp NOT NULL,
        "created_epoch" bigint NOT NULL,
        "updated_at" timestamp NOT NULL,
        "updated_epoch" bigint NOT NULL,
        "deleted_at" timestamp,
        "archived_at" timestamp,
        "created_by_id" uuid,
        "created_by_type" smallint,
        "created_by_base_ck" uuid,
        "updated_by_id" uuid,
        "updated_by_type" smallint,
        "updated_by_base_ck" uuid,
        "origin_id" uuid NOT NULL,
        "origin_ck" uuid NOT NULL,
        "origin_type" smallint NOT NULL,
        "origin_bench_id" uuid NOT NULL,
        "origin_base_ck" uuid,
        "origin_base_bench_id" uuid,
        "path" jsonb,
        "reply_to_id" uuid,
        "reply_to_base_ck" uuid,
        "title" varchar,
        "text" jsonb,
        "value_packed" jsonb,
        "secret_value_packed" jsonb,
        "is_pinned" boolean NOT NULL DEFAULT false,
        "block_id" uuid,
        "block_ck" uuid,
        "block_bench_id" uuid,
        "step_id" uuid,
        "step_ck" uuid,
        "step_bench_id" uuid,
        "session_id" uuid,
        "run_id" uuid,
        "run_base_ck" uuid,
        "run_root_id" uuid,
        "run_root_base_ck" uuid,
        "client_id" uuid,
        "machine_id" uuid,
        "server_id" uuid,
        "user_id" uuid
    )
    """
    )

    # bench_session
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
    await cur.execute(
        'CREATE INDEX "bench_session_bench_idx_status" ON bench_session USING BTREE (status)'
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

    # bench_signal
    await cur.execute(
        'CREATE INDEX "bench_signal_bench_idx_created_at" ON bench_signal USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_signal_bench_idx_created_epoch" ON bench_signal USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_signal_bench_idx_package_id_created_at" ON bench_signal USING BTREE (package_id, created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_signal_bench_idx_package_id_created_epoch" ON bench_signal USING BTREE (package_id, created_epoch)'
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

    # bench_notification
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_created_at" ON bench_notification USING BTREE (created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_created_epoch" ON bench_notification USING BTREE (created_epoch)'
    )
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_package_id_created_at" ON bench_notification USING BTREE (package_id, created_at)'
    )
    await cur.execute(
        'CREATE INDEX "bench_notification_bench_idx_package_id_created_epoch" ON bench_notification USING BTREE (package_id, created_epoch)'
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


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
