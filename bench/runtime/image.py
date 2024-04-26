import platform
import sys

from bench.proto.wire import ServerImageDependencyData, ServerImageData


def get_actual_image() -> ServerImageData:
    dependencies: list[ServerImageDependencyData] = []

    # load packages from requirements-server.txt
    with open("requirements-server.txt") as f:
        for line in f:
            line = line.strip()
            if line.startswith("#") or not line:
                continue
            name, version = line.split("==")
            dependency = ServerImageDependencyData(name=name, version=version)
            dependencies.append(dependency)

    osinfo = platform.uname()
    version = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
    return ServerImageData(
        language="python",
        version=version,
        platform=f"{osinfo.system} {osinfo.release}".split("-")[0],
        packages=dependency,
    )
