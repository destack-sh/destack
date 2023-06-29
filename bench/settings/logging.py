from bench.utils.logging import LOGGING, configure_logging

LOGGING  # noqa re-export for django

configure_logging(apply_logging=False, apply_structlog=True)
