def test_map_function_cls():
    pass


def test_map_function_callable():
    def upper_case(text: str) -> str:
        """Transforms text to uppercase"""
        return text.upper()
