import signal
import sys

from django.apps import AppConfig


class BenchConfig(AppConfig):
    name = "bench"
    verbose_name = "The Bench"

    def ready(self) -> None:
        from bench.executor import executor

        executor.start()

        def stop_int(*args):
            executor.stop()
            raise KeyboardInterrupt

        def stop_term(*args):
            executor.stop()
            sys.exit(0)

        signal.signal(signal.SIGINT, stop_int)
        signal.signal(signal.SIGTERM, stop_term)
