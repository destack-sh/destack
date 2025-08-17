from os import getcwd

from .core import (
    DESTACK_RS_SRC_PATH,
    RustFile,
    raw_path_to_local_path,
    raw_path_to_source_path,
)
from .parse import parse_rust_file


def scan_files() -> dict[str, RustFile]:
    """Scan for existing files (by source path like `simulation/geometry/vector.rs`)."""
    files: dict[str, RustFile] = {}
    all_rs_paths = list(DESTACK_RS_SRC_PATH.glob("**/*.rs"))
    assert DESTACK_RS_SRC_PATH.exists(), (
        f"DESTACK_RS_PATH {DESTACK_RS_SRC_PATH} not found {getcwd()}"
    )
    assert len(all_rs_paths) > 0, f"no .rs files found in {DESTACK_RS_SRC_PATH} from {getcwd()}"
    for path in all_rs_paths:
        local_path = raw_path_to_local_path(path)
        source_path = raw_path_to_source_path(path)
        try:
            rust_file = parse_rust_file(
                source=path.read_text(),
                local_path=local_path,
                source_path=source_path,
            )
        except Exception as e:
            raise RuntimeError(f"failed to parse {path.absolute()}: {e}") from e
        files[rust_file.local_path] = rust_file
    return files
