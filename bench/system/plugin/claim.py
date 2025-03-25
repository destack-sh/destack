from typing import override

from bench.language import Claim, ClaimStatus, NodeMode, NodeType, Session, bittuple
from bench.system.host import Commit, HostPlugin


class ClaimPlugin(HostPlugin[Claim]):
    """A Provisioner that satisfies Claims."""

    watch_types = bittuple(NodeType.CLAIM)

    @override
    async def start(self) -> None:
        pass

    @override
    async def on_commit(self, session: Session, commit: Commit[Claim]) -> None:
        # nocheckin: provision Claims better
        for claim in commit.added:
            if claim.status == ClaimStatus.REQUESTED and claim.mode < NodeMode.TEMPLATE:
                resource_template = claim.target_template
                assert resource_template is not None, f"claim {claim!r} has no target template"
                resource = resource_template.instance(detach=True)
                resource.parent = claim.parent
                claim.target = resource
                claim.status = ClaimStatus.OPEN
                claim.opened_at = self.oracle.utc()
                session._create(resource)
        await session.commit()
