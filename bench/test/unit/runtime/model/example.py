def test_example_run():
    """Just import the examples to make sure they work."""
    from bench.runtime.model.example import EXAMPLES

    assert EXAMPLES

    for example in EXAMPLES:
        print("=" * 80)  # noqa: T201
        print("# ", (example.title or "EXAMPLE").upper())  # noqa: T201
        print("# ", example.text)  # noqa: T201
        print("=" * 80)  # noqa: T201
        print("# Request")  # noqa: T201
        print(example.request)  # noqa: T201
        print("# Response")  # noqa: T201
        print(example.response)  # noqa: T201
        print()  # noqa: T201
