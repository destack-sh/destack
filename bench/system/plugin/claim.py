from typing import override

import structlog
from opentelemetry import trace

from bench.language import (
    RESOURCE_NODE_TYPES,
    Claim,
    ClaimStatus,
    IsInstantiable,
    IsProvisionable,
    NodeMode,
    NodeType,
    bittuple,
)
from bench.system.host import Commit, DeferredHostPlugin

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


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
    async def post_commit_deferred(self, commit: Commit[Claim]) -> None:
        pending_claims: list[Claim] = [
            claim
            for claim in commit.added
            if claim.status.is_pending and claim.mode < NodeMode.TEMPLATE
        ]

        # provision Claims/Resources
        if pending_claims:
            async with self.host.session(commit=True) as session:
                for claim in pending_claims:
                    # figure out where to add the resource
                    # TODO :Robustness: moving and reassigning Nodes in ClaimPlugin feels funky?
                    #  (because ._supergraph is different so we have to shuffle around...)
                    parent = claim.parent
                    assert parent is not None, f"claim {claim!r} has no parent"
                    parent._untrack_rec()
                    parent._track_rec(session)

                    # add target resource if needed
                    target_template = claim.target_template
                    if isinstance(target_template, IsInstantiable):
                        target = target_template.instance(detach=True)
                        target.move(to=parent)
                        target._supergraph = parent._supergraph
                        target.claimed_by = claim
                        claim.target = target
                        logger.info(
                            "claim.provision",
                            claim=claim,
                            target=target,
                            target_template=target_template,
                        )

                    # open the claim
                    claim.status = ClaimStatus.OPEN
                    claim.opened_at = self.oracle.utc()

        # decommission Resources
        decommissioned_claims: list[Claim] = [
            claim
            for claim in commit.removed
            if claim.mode < NodeMode.TEMPLATE
            and claim.target_ptr is not None
            and claim.target_ptr.node_type in RESOURCE_NODE_TYPES
            and claim.status.is_active
        ]
        if decommissioned_claims:
            async with self.host.session(commit=True) as session:
                for claim in decommissioned_claims:
                    assert claim.target_ptr is not None, f"claim {claim!r} has no target"
                    target = claim.target
                    if target is None:
                        target = await claim.target_ptr.get()
                    claim.status = ClaimStatus.CLOSED
                    if isinstance(target, IsProvisionable) and target.status.is_extant:
                        target.decommission()
                        logger.info("claim.decommission", claim=claim, target=target)
