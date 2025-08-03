from typing import TYPE_CHECKING

import pytest
from pytest_asyncio import is_async_test

if TYPE_CHECKING:
    pass

# use session scoped asyncio event loop
pytestmark = pytest.mark.asyncio(loop_scope="session")


_is_test_setup: bool = False


def _setup_test_env():
    """Initializes the test environment. Must be called before importing any destack modules."""

    global _is_test_setup
    if _is_test_setup:
        return

    from destack.core.utils.env import ENV, Env, setup_env

    assert ENV == Env.TEST, f"must run in {Env.TEST} (got: {ENV!r})"

    setup_env()

    _is_test_setup = True


_setup_test_env()


def pytest_addoption(parser):
    pass


def pytest_configure(config):
    pass


def pytest_collection_modifyitems(items):
    # ensure all async tests are run in session scope (with the same event loop)
    # see https://pytest-asyncio.readthedocs.io/en/latest/how-to-guides/run_session_tests_in_same_loop.html
    pytest_asyncio_tests = (item for item in items if is_async_test(item))
    session_scope_marker = pytest.mark.asyncio(scope="session")
    for async_test in pytest_asyncio_tests:
        async_test.add_marker(session_scope_marker)


def pytest_sessionstart(session: pytest.Session):
    # ideally we clean up dangling test databases (and other resources) here, but this also
    #  gets called for every test worker when running via pytest-xdist, so.. hmm
    pass


def pytest_sessionfinish(session: pytest.Session, exitstatus):
    pass


from destack.test.fixtures import *  # noqa: E402, F403
