from bench.backend.execute import execute
from bench.language.parse import index_module, parse_string


def test_execute_single_code():
    BENCH = """
--- test.bench ---
code function:
```python
return 5
```
"""
    module = parse_string(BENCH)
    idx = index_module(module)
    code = idx.statement(".test::function").content
    assert execute(code) == 5
