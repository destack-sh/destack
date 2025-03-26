from typing import override

from bench.language import Claim, ClaimStatus, NodeMode, NodeType, bittuple
from bench.system.host import Commit, DeferredHostPlugin


class ClaimPlugin(DeferredHostPlugin[Claim]):
    """
    A Provisioner that satisfies Claims.
    NOTE :Incomplete: right now we just create a new Resource for every unsatisfied Claim
    """

    watch_types = bittuple(NodeType.CLAIM)

    @override
    async def start(self) -> None:
        await super().start()

    @override
    def filter_commit(self, commit: Commit) -> bool:
        return any(
            claim.status.is_pending and claim.mode < NodeMode.TEMPLATE for claim in commit.added
        )

    @override
    async def on_commit_deferred(self, commit: Commit[Claim]) -> None:
        pending_claims: list[Claim] = [
            claim
            for claim in commit.added
            if claim.status.is_pending and claim.mode < NodeMode.TEMPLATE
        ]
        if not pending_claims:
            return  # nothing to do

        # provision Claims
        async with self.host.session(commit=True) as session:
            for claim in pending_claims:
                # figure out where to add the resource
                # TODO :Robustness: moving and reassigning Nodes feels funky?
                #  (and assigning ._supergraph directly feels extra funky?)
                parent = claim.parent
                assert parent is not None, f"claim {claim!r} has no parent"
                parent._untrack_rec()
                parent._track_rec(session)

                # add the resource
                resource_template = claim.target_template
                assert resource_template is not None, f"claim {claim!r} has no target template"
                resource = resource_template.instance(detach=True)
                resource.move(to=parent)
                resource._supergraph = parent._supergraph

                # open the claim
                claim.target = resource
                claim.status = ClaimStatus.OPEN
                claim.opened_at = self.oracle.utc()
