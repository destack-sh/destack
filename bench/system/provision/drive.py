from typing import override

from bench.language import Drive, NodeType
from bench.language.bench import ResourceStatus
from bench.system.provision.provisioner import Provisioner
from bench.utils.func import bittuple


class S3DriveProvisioner(Provisioner[Drive, Drive]):
    """Provision Drives with an S3-compatible API."""

    watch_types = bittuple(NodeType.DRIVE)
    resource_type = NodeType.DRIVE

    # NOTE: don't actually need to do anything since we share drives between Benches (per region)

    @override
    async def _do_provision(self, resource: Drive):
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Drive):
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED
