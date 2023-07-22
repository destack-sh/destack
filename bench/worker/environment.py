import sys

from bench.language.wire import EnvironmentData


def _collect_environment() -> EnvironmentData:
    packages: dict[str, str] = {}

    # load packages from requirements-worker.txt
    with open("requirements-worker.txt") as f:
        for line in f:
            line = line.strip()
            if line.startswith("#") or not line:
                continue
            name, version = line.split("==")
            packages[name] = version

    version = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
    return EnvironmentData(
        language="python",
        version=version,
        platform=sys.platform,
        packages=packages,
    )


WORKER_ENVIRONMENT_DATA = _collect_environment()
