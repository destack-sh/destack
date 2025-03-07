# This migration was automatically generated on 2025.03.07. Edit as needed.
import psycopg

ID = 42
VERSION = "2025.03.07.4"
HAS_GLOBAL = True
HAS_REGIONAL = True
HAS_LOCAL = True


#
# Global DB
#


async def upgrade_global(cur: psycopg.AsyncCursor):
    # bench_bench
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "main_package_ck"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_bench" DROP COLUMN "updated_by_ck"')

    # bench_client
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "space_ck"')
    await cur.execute('ALTER TABLE "bench_client" DROP COLUMN "updated_by_ck"')

    # bench_handle
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_handle" DROP COLUMN "updated_by_ck"')

    # bench_user
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_user" DROP COLUMN "updated_by_ck"')

    # bench_organization
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_organization" DROP COLUMN "updated_by_ck"')

    # bench_team
    await cur.execute('ALTER TABLE "bench_team" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_team" DROP COLUMN "updated_by_ck"')


async def downgrade_global(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Regional DB
#


async def upgrade_regional(cur: psycopg.AsyncCursor):
    # bench_scaler
    await cur.execute('ALTER TABLE "bench_scaler" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_scaler" DROP COLUMN "updated_by_ck"')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_store" DROP COLUMN "updated_by_ck"')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_machine" DROP COLUMN "updated_by_ck"')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_browser" DROP COLUMN "updated_by_ck"')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_file" DROP COLUMN "updated_by_ck"')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_stream" DROP COLUMN "updated_by_ck"')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_secret" DROP COLUMN "updated_by_ck"')

    # bench_scaler
    await cur.execute('ALTER TABLE "bench_scaler" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_scaler" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_scaler" ADD COLUMN "template_at" timestamp')

    # bench_store
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_store" ADD COLUMN "template_at" timestamp')

    # bench_machine
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_machine" ADD COLUMN "template_at" timestamp')

    # bench_browser
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_browser" ADD COLUMN "template_at" timestamp')

    # bench_file
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_file" ADD COLUMN "template_at" timestamp')

    # bench_stream
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_stream" ADD COLUMN "template_at" timestamp')

    # bench_secret
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_secret" ADD COLUMN "template_at" timestamp')


async def downgrade_regional(cur: psycopg.AsyncCursor):
    raise NotImplementedError


#
# Local DB
#


async def upgrade_local(cur: psycopg.AsyncCursor):
    # bench_dependency
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "depends_on_packages_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_dependency" DROP COLUMN "updated_by_ck"')

    # bench_block
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_block" DROP COLUMN "updated_by_ck"')

    # bench_choice
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_choice" DROP COLUMN "updated_by_ck"')

    # bench_field
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "oneof_base_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "oneof_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_field" DROP COLUMN "updated_by_ck"')

    # bench_action
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "tool_ck"')
    await cur.execute('ALTER TABLE "bench_action" DROP COLUMN "updated_by_ck"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "node_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "node_base_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_view" DROP COLUMN "updated_by_ck"')

    # bench_channel
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_channel" DROP COLUMN "updated_by_ck"')

    # bench_trigger
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "interruption_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "interruption_base_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "run_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "run_root_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_trigger" DROP COLUMN "updated_by_ck"')

    # bench_thread
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "channel_ck"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "created_from_base_ck"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "scope_ck"')
    await cur.execute('ALTER TABLE "bench_thread" DROP COLUMN "updated_by_ck"')

    # bench_database
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_database" DROP COLUMN "updated_by_ck"')

    # bench_role
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_role" DROP COLUMN "updated_by_ck"')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "channel_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "inspection_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "inspection_base_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "run_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_space" DROP COLUMN "updated_by_ck"')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "channel_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "message_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "message_base_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "nodes_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "nodes_base_ck"')
    await cur.execute('ALTER TABLE "bench_notification" DROP COLUMN "updated_by_ck"')

    # bench_membership
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "to_ck"')
    await cur.execute('ALTER TABLE "bench_membership" DROP COLUMN "updated_by_ck"')

    # bench_invite
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "to_ck"')
    await cur.execute('ALTER TABLE "bench_invite" DROP COLUMN "updated_by_ck"')

    # bench_run_span
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "interruption_base_ck"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "nodes_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "nodes_base_ck"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "root_base_ck"')
    await cur.execute('ALTER TABLE "bench_run_span" DROP COLUMN "updated_by_ck"')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "channel_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "clazz_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "created_by_ck"')
    await cur.execute(
        'ALTER TABLE "bench_message" DROP COLUMN "created_interruption_base_bench_id"'
    )
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "created_interruption_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "created_run_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "created_run_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "forwarded_from_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "forwarded_from_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "nodes_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "nodes_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "reply_to_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "reply_to_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_base_bench_id"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "run_root_base_ck"')
    await cur.execute('ALTER TABLE "bench_message" DROP COLUMN "updated_by_ck"')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "action_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "channel_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "flow_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "incoming_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "interruption_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "kit_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "link_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "page_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "root_base_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "trigger_ck"')
    await cur.execute('ALTER TABLE "bench_run" DROP COLUMN "updated_by_ck"')

    # bench_interruption
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "action_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "cancel_trigger_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "complete_trigger_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "flow_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "link_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "message_base_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "page_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "root_base_ck"')
    await cur.execute('ALTER TABLE "bench_interruption" DROP COLUMN "updated_by_ck"')

    # bench_session
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_session" DROP COLUMN "updated_by_ck"')

    # bench_log
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_log" DROP COLUMN "updated_by_ck"')

    # bench_flow
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "roles_ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_flow" DROP COLUMN "updated_by_ck"')

    # bench_link
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "source_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "target_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_link" DROP COLUMN "updated_by_ck"')

    # bench_kit
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_kit" DROP COLUMN "updated_by_ck"')

    # bench_option
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_option" DROP COLUMN "updated_by_ck"')

    # bench_package
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "base_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "main_channel_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_package" DROP COLUMN "updated_by_ck"')

    # bench_class
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_class" DROP COLUMN "updated_by_ck"')

    # bench_tag
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_tag" DROP COLUMN "updated_by_ck"')

    # bench_page
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "ck"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "parent_ck"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "template_ck"')
    await cur.execute('ALTER TABLE "bench_page" DROP COLUMN "updated_by_ck"')

    # bench_task
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "clazz_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "definition_bench_id"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "definition_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "implemented_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "interruption_base_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "tags_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "target_ck"')
    await cur.execute('ALTER TABLE "bench_task" DROP COLUMN "updated_by_ck"')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "created_by_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "implemented_by_base_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "owned_by_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "package_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "parent_base_ck"')
    await cur.execute('ALTER TABLE "bench_plan" DROP COLUMN "updated_by_ck"')

    # bench_view
    await cur.execute('ALTER TABLE "bench_view" ADD COLUMN "node_base_id" uuid')

    # bench_space
    await cur.execute('ALTER TABLE "bench_space" ADD COLUMN "inspection_base_id" uuid')

    # bench_message
    await cur.execute('ALTER TABLE "bench_message" ADD COLUMN "nodes_base_id" uuid[]')

    # bench_notification
    await cur.execute('ALTER TABLE "bench_notification" ADD COLUMN "nodes_base_id" uuid[]')

    # bench_run
    await cur.execute('ALTER TABLE "bench_run" ADD COLUMN "plan_ck" uuid')

    # bench_run_span
    await cur.execute('ALTER TABLE "bench_run_span" ADD COLUMN "nodes_base_id" uuid[]')

    # bench_plan
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "ck" uuid NOT NULL')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "template_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "template_ck" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "template_bench_id" uuid')
    await cur.execute('ALTER TABLE "bench_plan" ADD COLUMN "template_at" timestamp')


async def downgrade_local(cur: psycopg.AsyncCursor):
    raise NotImplementedError
