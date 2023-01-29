from bench.language.parse import index_module, parse_string
from bench.runtime.bpl import parse_bpl
from bench.runtime.execute import execute_sync, instantiate


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
    code = instantiate(idx.symbol(".test:function"), idx)
    assert execute_sync(code) == 5


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
    code = instantiate(idx.symbol(".test:function"), idx)
    assert execute_sync(code) == 10


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
    code = instantiate(idx.symbol(".test:function"), idx)
    assert execute_sync(code, {"val": 2}) == 10


def test_parse_bpl():
    bpl = r"""
pragma(model="gpt2", n=1, z=None)
"Count the animals in the {zoo}."
# some comment
animals = []
for _ in range(max_animals):
    " - [animal: string]\n"
    animals.append(animal)
return animals
    """
    prompt = parse_bpl(bpl, {})
    print(prompt.python_code)
