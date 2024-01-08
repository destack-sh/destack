from contextlib import asynccontextmanager

from bench.language.session import Session


@asynccontextmanager
async def global_session(commit: bool = False) -> "Session":
    """Get a global session."""
    session = Session(
        parent=None,
    )
    try:
        yield session
    finally:
        if commit:
            await session.commit()
        else:
            await session.rollback()
