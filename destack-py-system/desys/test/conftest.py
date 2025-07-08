import pytest

from desys.test.fixtures import *  # noqa: F403

# use session scoped asyncio event loop
pytestmark = pytest.mark.asyncio(loop_scope="session")
