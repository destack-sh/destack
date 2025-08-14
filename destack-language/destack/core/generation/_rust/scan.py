from os import getcwd
from pathlib import Path

from .core import RustFile
from .parse import parse_rust_file

DESTACK_RS_PATH = Path("../destack-rs/destack/src")


def scan_files() -> dict[str, RustFile]:
    """Scan for existing files (by normalized path like `destack.simulation.geometry.vector.Vector2`)."""
    files: dict[str, RustFile] = {}
    all_rs_paths = list(DESTACK_RS_PATH.glob("**/*.rs"))
    assert DESTACK_RS_PATH.exists(), f"DESTACK_RS_PATH {DESTACK_RS_PATH} not found {getcwd()}"
    assert len(all_rs_paths) > 0, f"no .rs files found in {DESTACK_RS_PATH} from {getcwd()}"
    for path in all_rs_paths:
        normalized_path = str(path.relative_to(DESTACK_RS_PATH).with_suffix(""))
        normalized_path = normalized_path.replace("/", ".")
        normalized_path = "destack." + normalized_path

        try:
            rust_file = parse_rust_file(
                source=path.read_text(),
                raw_path=str(path),
                normalized_path=normalized_path,
            )
        except Exception as e:
            raise RuntimeError(f"failed to parse {path.absolute()}: {e}") from e
        files[rust_file.normalized_path] = rust_file
    return files
