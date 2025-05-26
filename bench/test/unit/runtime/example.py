def test_print_examples():
    """Just import the examples to make sure they work."""
    from bench.runtime.agent.example import EXAMPLES

    assert EXAMPLES

    for example in EXAMPLES:
        print("=" * 32)  # noqa: T201
        print(example.title)  # noqa: T201
        print("=" * 32)  # noqa: T201
        print(example.code)  # noqa: T201
        print("=" * 32)  # noqa: T201
