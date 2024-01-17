def pytest_configure(config):
    from bench.utils.env import setup_dotenv

    setup_dotenv()
