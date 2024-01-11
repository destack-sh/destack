from contextlib import asynccontextmanager
from datetime import datetime

import structlog

from bench.language import Session
from bench.language.builtin import _active_session
from bench.sql.client import async_pg_cursor

logger = structlog.get_logger(__name__)


@asynccontextmanager
async def detached_session(commit: bool = False, read_only: bool = False) -> "Session":
    """Get a global session."""
    assert not read_only or not commit, "read_only and commit are mutually exclusive"
    assert _active_session.get() is None, f"already in active session {_active_session.get()}"

    async with async_pg_cursor(local_pg_name=None) as global_pg_cursor:
        session = Session(parent=None, _global_pg_cursor=global_pg_cursor)
        _active_session.set(session)
        try:
            yield session
            if commit:
                await session.commit()
            elif session.has_regular_edits:
                if read_only:
                    raise RuntimeError(f"read_only session {session!r} has edits")
                logger.warning("session.discard", session=session)
        finally:
            session.closed_at = datetime.utcnow()  # pretend close to prevent further use
            _active_session.set(None)
