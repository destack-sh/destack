from bench.backend.execute import execute, instantiate
from bench.language.parse import index_module, parse_string


def test_execute_single_code():
    module = parse_string(
        """
--- test.instruct ---
code function:
```python
return 5
```
    schema _:
    `{ input: null, output: number }`
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

code function:
```python
return val * context['unwieldy name']
```
    schema _:
    `{ input: null, output: number }`
"""
    )
    idx = index_module(module)
    code = instantiate(idx.statement(".test::function"), idx)
    assert execute(code) == 10


def test_execute_single_code_with_schema():
    module = parse_string(
        """
--- test.instruct ---
code function:
```python
return 5 * val
```
    schema _:
    `{ input: { val: number }, output: number }`
"""
    )
    idx = index_module(module)
    code = instantiate(idx.statement(".test::function"), idx)
    assert execute(code, {"val": 2}) == 10
