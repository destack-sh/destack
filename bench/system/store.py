from typing import TYPE_CHECKING, override

import structlog
from opentelemetry import trace

from bench.language import Bench, Drive, ResourceStatus, Store
from bench.language.const import VERSION, NodeType
from bench.sql.client import pg_store_connection
from bench.sql.engine import sqlstr
from bench.sql.migration import sql_migrate
from bench.system.core import HostApi
from bench.system.neon import NeonApi
from bench.system.provisioner import Provisioner
from bench.utils.env import ENV, IS_DEV, IS_TEST
from bench.utils.func import bittuple

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class StoreProvisioner(Provisioner[Store, Store]):
    """Base for provisioning Stores."""

    watch_types = bittuple(NodeType.STORE)
    provision_types = bittuple(NodeType.STORE)

    async def _do_migrate(self, resource: Store):
        """Migrate the store to its indicated 'version'."""
        assert resource.version, f"{resource!r} has no version"
        async with pg_store_connection(resource) as cur:
            await sql_migrate(
                cur,
                target=resource.version,
                is_global=False,
                store=resource,
                oracle=self.host.oracle,
            )
            await cur.connection.commit()
        async with self.host.session(autocommit=True):
            resource.current_version = resource.version

    @override
    async def _do_update(self, resource: Store):
        # auto-migrate if version changed
        if resource.version != resource.current_version:
            await self._do_migrate(resource)


class NeonStoreProvisioner(StoreProvisioner):
    """Provision Stores with the Neon API."""

    def __init__(self, host: "HostApi", bench: Bench, neon_api: "NeonApi"):
        super().__init__(host, bench)
        self._neon_api = neon_api

    @override
    async def _do_provision(self, resource: Store):
        # assign a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self.host.session(autocommit=True):
                resource.external_name = f"{ENV}-{resource.bench_id}"
        # create postgres database ('project')
        neon_project = await self._neon_api.create_project(
            name=resource.external_name, region=resource.region, pg_version=16
        )
        async with self.host.session(autocommit=True):
            resource.external_id = neon_project.project_id
            resource.connection_uri = neon_project.connection_uri
            if not resource.version:
                resource.version = VERSION
            resource.status = ResourceStatus.HEALTHY
        # migrate it immediately
        await self._do_migrate(resource)

    @override
    async def _do_decommission(self, resource: Store):
        assert resource.external_id, f"{resource!r} has no external ID"
        await self._neon_api.delete_project(project_id=resource.external_id)
        async with self.host.session(autocommit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class LocalhostStoreProvisioner(StoreProvisioner):
    """Provision Stores as local Postgres databases (in the existing database)."""

    @override
    async def _do_provision(self, resource: Store):
        assert IS_DEV or IS_TEST, f"cannot create localhost store in environment: {ENV!r}"
        # assign a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self.host.session(autocommit=True):
                resource.external_name = f"{ENV}-{resource.bench_id}"
        # create database through existing connection
        # (use same postgres instance as global store)
        async with pg_store_connection(self.host.global_store, autocommit=True) as cur:
            await cur.execute(sqlstr(f'CREATE DATABASE "{resource.external_name}"'))
        async with self.host.session(autocommit=True):
            connection_uri = self.host.global_store.connection_uri
            assert connection_uri, f"{self.host.global_store!r} has no connection URI"
            resource.connection_uri = f"{connection_uri.rsplit('/', 1)[0]}/{resource.external_name}"
            if not resource.version:
                resource.version = VERSION
            resource.status = ResourceStatus.HEALTHY
        # migrate it immediately
        await self._do_migrate(resource)

    @override
    async def _do_decommission(self, resource: Store):
        # drop database through existing connection
        async with pg_store_connection(self.host.global_store) as cur:
            await cur.execute(sqlstr(f'DROP DATABASE "{resource.external_name}"'))


class S3DriveProvisioner(Provisioner[Drive, Drive]):
    """Provision Drives with an S3-compatible API."""

    watch_types = bittuple(NodeType.DRIVE)
    provision_types = bittuple(NodeType.DRIVE)

    # NOTE: don't actually need to do anything since we share drives between Benches (per region)

    @override
    async def _do_provision(self, resource: Drive):
        async with self.host.session(autocommit=True):
            resource.status = ResourceStatus.HEALTHY

    @override
    async def _do_decommission(self, resource: Drive):
        async with self.host.session(autocommit=True):
            resource.status = ResourceStatus.DECOMMISSIONED
