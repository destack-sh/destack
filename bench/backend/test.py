from bench.backend.execute import execute, instantiate
from bench.language.parse import index_module, parse_string


def test_execute_single_code():
    module = parse_string(
        """
--- test.instruct ---
code function :: () -> number:
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
--- test.instruct ---
value val:
`5`

value 'unwieldy name':
`2`

code function :: () -> number:
```python
return val * context['unwieldy name']
```
"""
    )
    idx = index_module(module)
    code = instantiate(idx.statement(".test::function"), idx)
    assert execute(code) == 10


def test_execute_single_code_with_args():
    module = parse_string(
        """
--- test.instruct ---
code function :: (val: number) -> number:
```python
return 5 * val
```
"""
    )
    idx = index_module(module)
    code = instantiate(idx.statement(".test::function"), idx)
    assert execute(code, {"val": 2}) == 10
