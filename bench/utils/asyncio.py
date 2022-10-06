import asyncio


def ensure_event_loop():
    """
    Ensures that an event loop is running in the current thread.
    Use with care as this may interfere with other event loops.
    """
    try:
        if not asyncio.get_event_loop().is_running():
            asyncio.set_event_loop(asyncio.new_event_loop())
    except RuntimeError:
        # will be raised if no event loop is running and we're not in the main thread
        asyncio.set_event_loop(asyncio.new_event_loop())

    return asyncio.get_event_loop()
