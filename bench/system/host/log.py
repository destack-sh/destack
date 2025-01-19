from typing import Sequence, override

from bench.language import Session
from bench.proto import EditData

from .core import Commit, HostPlugin


class LogPlugin(HostPlugin):
    """Generate Logs for every Edit/Change."""

    @override
    async def on_commit_prepare(self, session: Session, commit: Commit) -> Sequence[EditData]:
        # nothing to do yet
        return ()
