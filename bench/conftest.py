import asyncio

import pytest


def pytest_configure(config):
    from bench.utils.env import setup_dotenv

    setup_dotenv()

    from bench.utils.logging import configure_logging

    configure_logging()

    from bench.language import _complete_bench_setup

    _complete_bench_setup()


@pytest.fixture(scope="package")
def event_loop():
    # ensure we have one global event loop, lest our async fixtures are fucked
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()
