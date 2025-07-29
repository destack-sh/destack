import pytest
from destack_system.test.fixtures import *  # noqa: F403

# use session scoped asyncio event loop
pytestmark = pytest.mark.asyncio(loop_scope="session")
