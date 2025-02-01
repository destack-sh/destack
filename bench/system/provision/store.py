from typing import TYPE_CHECKING, override

import structlog
from opentelemetry import trace

from bench.language import Bench, NodeArea, NodeType, ResourceStatus, Store
from bench.language.core import bittuple
from bench.sql import pg_connection, sql_migrate, sqlstr
from bench.utils.env import ENV, IS_DEV, IS_TEST

from .neon import neon_api
from .provisioner import Provisioner

if TYPE_CHECKING:
    from bench.system.host import HostService


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class StoreProvisioner(Provisioner[Store, Store]):
    """Base for provisioning Stores."""

    watch_types = bittuple(NodeType.STORE)
    provision_type = NodeType.STORE

    async def _do_migrate(self, resource: Store):
        """Migrate the store to its indicated 'version'."""
        assert resource.version, f"{resource!r} has no version"
        async with pg_connection(resource) as conn:
            await sql_migrate(
                conn.cursor,
                target=resource.version,
                area=NodeArea.LOCAL,
                store=resource,
                oracle=self.host.oracle,
            )
            await conn.commit()
        async with self.host.session(commit=True):
            resource.version = resource.target_version

    @override
    async def _do_update(self, resource: Store):
        # auto-migrate if version changed
        if resource.version != resource.target_version:
            await self._do_migrate(resource)


class NeonStoreProvisioner(StoreProvisioner):
    """Provision Stores with the Neon API."""

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._neon_api = neon_api

    @override
    async def _do_provision(self, resource: Store):
        # assign a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self.host.session(commit=True):
                resource.external_name = f"{ENV}-{resource.bench_id}"
        # create postgres database ('project')
        neon_project = await self._neon_api.create_project(
            name=resource.external_name, region=resource.region, pg_version=16
        )
        async with self.host.session(commit=True):
            resource.external_id = neon_project.project_id
            resource.connection_uri = neon_project.connection_uri
            resource.status = ResourceStatus.UP
        # migrate it immediately
        await self._do_migrate(resource)

    @override
    async def _do_decommission(self, resource: Store):
        assert resource.external_id, f"{resource!r} has no external ID"
        await self._neon_api.delete_project(project_id=resource.external_id)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class LocalhostStoreProvisioner(StoreProvisioner):
    """Provision Stores as local Postgres databases (in the existing database)."""

    @override
    async def _do_provision(self, resource: Store):
        assert IS_DEV or IS_TEST, f"cannot create localhost store in environment: {ENV!r}"
        # assign a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self.host.session(commit=True):
                resource.external_name = f"{ENV}-{resource.bench_id}"
        # create database through existing connection
        # (use same postgres instance as global store)
        async with pg_connection(self.host.global_store, autocommit=True) as conn:
            await conn.execute(sqlstr(f'CREATE DATABASE "{resource.external_name}"'))
        async with self.host.session(commit=True):
            connection_uri = self.host.global_store.connection_uri
            assert connection_uri, f"{self.host.global_store!r} has no connection URI"
            resource.connection_uri = f"{connection_uri.rsplit('/', 1)[0]}/{resource.external_name}"
            resource.status = ResourceStatus.UP
        # migrate it immediately
        await self._do_migrate(resource)

    @override
    async def _do_decommission(self, resource: Store):
        # drop database through existing connection
        async with pg_connection(self.host.global_store, autocommit=True) as conn:
            await conn.execute(sqlstr(f'DROP DATABASE "{resource.external_name}"'))
