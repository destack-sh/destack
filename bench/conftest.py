def pytest_configure(config):
    from bench.utils.env import setup_dotenv

    setup_dotenv()

    from bench.language import _complete_bench_setup

    _complete_bench_setup()
