import platform
import sys

from bench.proto.wire import DependencyData, EnvironmentData


def collect_environment() -> EnvironmentData:
    dependencies: list[DependencyData] = []

    # load packages from requirements-worker.txt
    with open("requirements-worker.txt") as f:
        for line in f:
            line = line.strip()
            if line.startswith("#") or not line:
                continue
            name, version = line.split("==")
            dependency = DependencyData(name=name, version=version)
            dependencies.append(dependency)

    osinfo = platform.uname()
    version = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
    return EnvironmentData(
        language="python",
        version=version,
        platform=f"{osinfo.system} {osinfo.release}".split("-")[0],
        packages=dependency,
    )


def install_environment(environment: EnvironmentData) -> None:
    raise NotImplementedError
