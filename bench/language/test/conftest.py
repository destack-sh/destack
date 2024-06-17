from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from bench.language import Bench


def bench_session(bench: "Bench", epoch: int = 0):
    from bench.language.session import Session
    from bench.system.core import GLOBAL_POSTGRES_ENGINE

    assert bench.main_branch is not None, f"{bench!r} has no main branch"
    return Session(
        parent=bench.main_branch.main_package, _engines=(GLOBAL_POSTGRES_ENGINE,), _epoch=epoch
    )


@pytest.fixture()
async def session():
    from bench.language import Bench

    bench = Bench(slug="test", name="Test")
    async with bench_session(bench) as session:
        yield session


@pytest.fixture(scope="module")
async def shared_session():
    from bench.language import Bench

    bench = Bench(slug="test", name="Test")
    async with bench_session(bench) as session:
        yield session
