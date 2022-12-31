from bench.backend.execute import execute, instantiate
from bench.language.parse import index_module, parse_string


def test_execute_single_code():
    module = parse_string(
        """
--- test.bench ---
code function:
```python
return 5
```
"""
    )
    idx = index_module(module)
    code = instantiate(idx.statement(".test::function"), idx)
    assert execute(code) == 5


def test_execute_single_code_with_context():
    module = parse_string(
        """
--- test.bench ---
value val:
`5`

value 'unwieldy name':
`1`

code function:
```python
return val * context['unwieldy name']
```
"""
    )
    idx = index_module(module)
    code = instantiate(idx.statement(".test::function"), idx)
    assert execute(code) == 5
