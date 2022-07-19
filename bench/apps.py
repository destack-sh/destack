from django.apps import AppConfig


class BenchConfig(AppConfig):
    name = "bench"
    verbose_name = "The Bench"

    def ready(self) -> None:
        from bench.executor import executor

        executor.start()
