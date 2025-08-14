from .generate import generate_files
from .scan import scan_files


def regenerate():
    """Regenerate the entire Rust library / runtime."""
    from destack import SCHEMA

    old_rust_files = scan_files()
    new_rust_files = generate_files(SCHEMA)
