import structlog
from psycopg import sql

from bench import models
from bench.language import libs, wire
from bench.language.const import INTERP_NODE_TYPES
from bench.storage.client import GLOBAL_RO_PASSWORD, GLOBAL_RO_USERNAME, async_pg_cursor
from bench.storage.mapping import update_pg_schema

logger = structlog.get_logger(__name__)


async def create_local_pg_database(project: models.Project, *, upsert: bool) -> None:
    """
    Creates the local Postgres database and corresponding roles/user for a project.
    """
    log = logger.bind(pg_name=project.pg_name, upsert=upsert)
    log.info("pg.create_local_database")

    # create database from the default one (if not exists)
    async with async_pg_cursor("postgres", autocommit=True) as cur:
        await cur.execute("SELECT 1 FROM pg_database WHERE datname = %s", (project.pg_name,))
        exists = bool(await cur.fetchone())
        if not exists:
            log.info("pg.create_local_database.create")
            await cur.execute(sql.SQL("CREATE DATABASE {}").format(sql.Identifier(project.pg_name)))
        else:
            log.info("pg.create_local_database.already_exists")

    # connect to local database and setup auth
    async with async_pg_cursor(project.pg_name, autocommit=False) as cur:
        # create 'owner' user (if not exists)
        log.info("pg.create_local_database.create_owner")
        await cur.execute("SELECT 1 FROM pg_roles WHERE rolname = %s", (project.pg_username,))
        exists = bool(await cur.fetchone())
        if not exists:
            log.info("pg.create_local_database.create_owner.create")
            await cur.execute(
                sql.SQL("CREATE USER {} WITH PASSWORD {}").format(
                    sql.Identifier(project.pg_username), sql.Literal(project.pg_password)
                ),
            )
        else:
            log.info("pg.create_local_database.create_owner.already_exists")
        # grant full regular CRUD access to 'owner' user (no trigger or such)
        log.info("pg.create_local_database.create_owner.grant")
        # revoke all privileges first (in case of upsert)
        if exists:
            await cur.execute(
                sql.SQL("REVOKE ALL ON SCHEMA PUBLIC FROM {}").format(
                    sql.Identifier(project.pg_username)
                )
            )
        # grant new
        await cur.execute(
            sql.SQL(
                "GRANT SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES ON ALL TABLES IN SCHEMA public TO {}"
            ).format(
                sql.Identifier(project.pg_username),
            )
        )

        # if public, add global read only user (if not exists)
        await cur.execute("SELECT 1 FROM pg_roles WHERE rolname = %s", (GLOBAL_RO_USERNAME,))
        exists = bool(await cur.fetchone())
        if project.visibility == models.ProjectVisibility.PUBLIC:
            if not exists:
                log.info("pg.create_local_database.create_global_ro.create")
                await cur.execute(
                    sql.SQL("CREATE USER {} WITH PASSWORD {}").format(
                        sql.Identifier(GLOBAL_RO_USERNAME), sql.Literal(GLOBAL_RO_PASSWORD)
                    ),
                )
            # grant read only
            log.info("pg.create_local_database.create_global_ro.grant")
            await cur.execute(
                sql.SQL("GRANT SELECT ON ALL TABLES IN SCHEMA public TO {}").format(
                    sql.Identifier(GLOBAL_RO_USERNAME),
                )
            )
        elif exists:
            log.info("pg.create_local_database.create_global_ro.remove")
            await cur.execute(
                sql.SQL("REVOKE ALL ON SCHEMA PUBLIC FROM {}").format(
                    sql.Identifier(GLOBAL_RO_USERNAME)
                )
            )

    log.info("pg.create_local_database.done")


async def update_pg_schema_from_db(project_v: models.ProjectVersion) -> None:
    # nocheckin: consolidate with update_os_schema_from_db? (at least in common call sites)
    from bench.models import packer

    logger.info("os.update_mappings", project_version=project_v)
    source = packer.pack_module(
        project_v, excluded=[models.Record, models.Trigger, models.ResolvedField, models.Issue]
    )
    module = wire.unpack_module(source.nodes, exclude=INTERP_NODE_TYPES, session=None)
    for dependency in libs.DEFAULT_MODULES.values():
        module.add_dependency(dependency)
    module.add_builtin(libs.symbolx_lib.files.get("builtins"))
    module._interp_rec()

    update_pg_schema(project_v.project.os_name, module)
