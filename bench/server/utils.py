from contextlib import asynccontextmanager

import structlog

from bench.language.builtin import _active_session
from bench.language.session import Session

logger = structlog.get_logger(__name__)


@asynccontextmanager
async def global_session(commit: bool = False) -> "Session":
    """Get a global session."""
    assert _active_session.get() is None, f"already in active session {_active_session.get()}"
    session = Session(parent=None)
    _active_session.set(session)
    try:
        yield session
        if commit:
            await session.commit()
        elif session.has_regular_edits:
            logger.warning("session.discard", session=session)
    finally:
        _active_session.set(None)
