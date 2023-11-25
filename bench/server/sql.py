import structlog
from psycopg import sql

from bench import models
from bench.sql.client import GLOBAL_RO_PASSWORD, GLOBAL_RO_USERNAME, async_pg_cursor
from bench.sql.engine import update_pg_schema

logger = structlog.get_logger(__name__)

USER_PRIVILEGES = "SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES"


async def create_local_pg_database(project: models.Project, *, upsert: bool) -> None:
    """
    Creates the local Postgres database and corresponding roles/user for a project.
    """
    log = logger.bind(pg_name=project.pg_name, upsert=upsert)
    log.info("pg.create_db")

    # create database from the default one (if not exists)
    async with async_pg_cursor("postgres", autocommit=True) as cur:
        await cur.execute("SELECT 1 FROM pg_database WHERE datname = %s", (project.pg_name,))
        exists = bool(await cur.fetchone())
        if not exists:
            log.info("pg.create_db.create")
            await cur.execute(sql.SQL("CREATE DATABASE {}").format(sql.Identifier(project.pg_name)))
        else:
            log.info("pg.create_db.already_exists")

    # connect to local database and setup auth
    async with async_pg_cursor(project.pg_name, autocommit=False) as cur:
        # create 'owner' user (if not exists)
        log.info("pg.create_db.create_owner", username=project.pg_username)
        await cur.execute("SELECT 1 FROM pg_roles WHERE rolname = %s", (project.pg_username,))
        exists = bool(await cur.fetchone())
        if not exists:
            log.info("pg.create_db.create_owner.create", username=project.pg_username)
            await cur.execute(
                sql.SQL("CREATE USER {} WITH PASSWORD {}").format(
                    sql.Identifier(project.pg_username), sql.Literal(project.pg_password)
                ),
            )
        else:
            log.info("pg.create_db.create_owner.already_exists", username=project.pg_username)
        # grant full regular CRUD access to 'owner' user (no trigger or such)
        log.info("pg.create_db.create_owner.grant")
        # grant new
        await cur.execute(
            sql.SQL("GRANT {} ON ALL TABLES IN SCHEMA public TO {}").format(
                sql.SQL(USER_PRIVILEGES),
                sql.Identifier(project.pg_username),
            )
        )
        # alter default privileges (to apply to all new tables)
        await cur.execute(
            sql.SQL("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT {} ON TABLES TO {}").format(
                sql.SQL(USER_PRIVILEGES),
                sql.Identifier(project.pg_username),
            )
        )

        # if public, add global read only user (if not exists)
        await cur.execute("SELECT 1 FROM pg_roles WHERE rolname = %s", (GLOBAL_RO_USERNAME,))
        exists = bool(await cur.fetchone())
        if project.visibility == models.ProjectVisibility.PUBLIC:
            if not exists:
                log.info("pg.create_db.create_global_ro.create")
                await cur.execute(
                    sql.SQL("CREATE USER {} WITH PASSWORD {}").format(
                        sql.Identifier(GLOBAL_RO_USERNAME), sql.Literal(GLOBAL_RO_PASSWORD)
                    ),
                )
            # grant read only
            log.info("pg.create_db.create_global_ro.grant")
            await cur.execute(
                sql.SQL("GRANT SELECT ON ALL TABLES IN SCHEMA public TO {}").format(
                    sql.Identifier(GLOBAL_RO_USERNAME),
                )
            )
        elif exists:
            log.info("pg.create_db.create_global_ro.remove")
            await cur.execute(
                sql.SQL("REVOKE ALL ON SCHEMA PUBLIC FROM {}").format(
                    sql.Identifier(GLOBAL_RO_USERNAME)
                )
            )

    log.info("pg.create_db.done")


async def update_pg_schema_from_db(project_v: models.ProjectVersion) -> None:
    from bench.server.runtime import interp_module

    logger.info("pg.update_mappings", project_version=repr(project_v))
    module, project = await interp_module(project_v.id)
    await update_pg_schema(project.pg_name, module)


async def delete_local_pg_database(project: models.Project) -> None:
    """
    Deletes the local Postgres database and corresponding roles/user for a project.
    """
    log = logger.bind(pg_name=project.pg_name)
    log.info("pg.delete_db")

    # connect to default database and drop the database
    async with async_pg_cursor("postgres", autocommit=True) as cur:
        log.info("pg.delete_db.drop", username=project.pg_username)
        await cur.execute(
            sql.SQL("DROP DATABASE IF EXISTS {}").format(sql.Identifier(project.pg_name))
        )

    log.info("pg.delete_db.done")
