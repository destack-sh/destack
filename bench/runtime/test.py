from bench.language.parse import index_module, parse_string
from bench.runtime.execute import execute, instantiate


def test_execute_single_code():
    module = parse_string(
        """
--- test.x ---
code function :: () -> number:
```python
return 5
```
"""
    )
    idx = index_module(module)
    code = instantiate(idx.statement(".test:function"), idx)
    assert execute(code) == 5


def test_execute_single_code_with_context():
    module = parse_string(
        """
--- test.x ---
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
    code = instantiate(idx.statement(".test:function"), idx)
    assert execute(code) == 10


def test_execute_single_code_with_args():
    module = parse_string(
        """
--- test.x ---
code function :: (val: number) -> number:
```python
return 5 * val
```
"""
    )
    idx = index_module(module)
    code = instantiate(idx.statement(".test:function"), idx)
    assert execute(code, {"val": 2}) == 10
