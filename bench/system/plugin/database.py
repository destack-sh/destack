from typing import TYPE_CHECKING, override

import structlog
from opentelemetry import trace

from bench.language import VERSION, Bench, Database, NodeArea, NodeType, ResourceStatus, bittuple
from bench.utils.env import ENV, IS_DEV, IS_TEST

from .neon import neon_api
from .provisioner import Provisioner

if TYPE_CHECKING:
    from bench.system.host import HostService


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class DatabaseProvisioner(Provisioner[Database, Database]):
    """Base for provisioning Databases."""

    watch_types = bittuple(NodeType.DATABASE)
    provision_type = NodeType.DATABASE

    async def _do_migrate(self, resource: Database):
        """Migrate the database to its indicated 'version'."""
        assert resource.version, f"{resource!r} has no version"
        async with pg_connection(resource, owner=self) as conn:
            await sql_migrate(
                conn,
                target=resource.version,
                area=NodeArea.LOCAL_POSTGRES,
                database=resource,
                oracle=self.host.oracle,
            )
            await conn.commit()
        async with self.host.session(commit=True):
            resource.version = VERSION

    @override
    async def _do_update(self, resource: Database):
        # auto-migrate if version changed
        if resource.version != VERSION:
            await self._do_migrate(resource)


class NeonDatabaseProvisioner(DatabaseProvisioner):
    """Provision Databases with the Neon API."""

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._neon_api = neon_api

    @override
    async def _do_provision(self, resource: Database):
        # assign a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self.host.session(commit=True):
                resource.external_name = f"{ENV}-{resource.bench_id}"
        # create postgres table ('project')
        neon_project = await self._neon_api.create_project(
            name=resource.external_name, region=resource.region, pg_version=16
        )
        async with self.host.session(commit=True):
            resource.external_id = neon_project.project_id
            resource.sql_url = neon_project.sql_url
            self._set_resource_status(resource, ResourceStatus.AVAILABLE)
        # migrate it immediately
        await self._do_migrate(resource)

    @override
    async def _do_decommission(self, resource: Database):
        assert resource.external_id, f"{resource!r} has no external ID"
        await self._neon_api.delete_project(project_id=resource.external_id)
        async with self.host.session(commit=True):
            self._set_resource_status(resource, ResourceStatus.OFFLINE)


class LocalhostDatabaseProvisioner(DatabaseProvisioner):
    """Provision Databases as local Postgres tables (in the existing table)."""

    @override
    async def _do_provision(self, resource: Database):
        assert IS_DEV or IS_TEST, f"cannot create localhost database in environment: {ENV!r}"
        # assign a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self.host.session(commit=True):
                resource.external_name = f"{ENV}-{resource.bench_id}"
        # create table through existing connection
        # (use same postgres instance as global database)
        async with pg_connection(self.host.global_database, owner=self, autocommit=True) as conn:
            await conn.execute(sqlstr(f'CREATE TABLE "{resource.external_name}"'))
        async with self.host.session(commit=True):
            sql_url = self.host.global_database.sql_url
            assert sql_url, f"{self.host.global_database!r} has no SQL URL"
            resource.sql_url = f"{sql_url.rsplit('/', 1)[0]}/{resource.external_name}"
            self._set_resource_status(resource, ResourceStatus.AVAILABLE)
        # migrate it immediately
        await self._do_migrate(resource)

    @override
    async def _do_decommission(self, resource: Database):
        # drop table through existing connection
        async with pg_connection(self.host.global_database, owner=self, autocommit=True) as conn:
            await conn.execute(sqlstr(f'DROP TABLE "{resource.external_name}"'))
