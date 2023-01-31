from bench.language.parse import parse_string
from bench.runtime.bpl import parse_bpl
from bench.runtime.execute import instantiate, run_sync


def test_execute_single_code():
    module, idx = parse_string(
        """
--- test.x ---
code function :: () -> number:
```python
return 5
```
"""
    )
    code = instantiate(idx.symbol(".test:function"))
    assert run_sync(code) == 5


def test_execute_single_code_with_context():
    module, idx = parse_string(
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
    code = instantiate(idx.symbol(".test:function"))
    assert run_sync(code) == 10


def test_execute_single_code_with_args():
    module, idx = parse_string(
        """
--- test.x ---
code function :: (val: number) -> number:
```python
return 5 * val
```
"""
    )
    code = instantiate(idx.symbol(".test:function"))
    assert run_sync(code, {"val": 2}) == 10


def test_parse_bpl():
    bpl = r"""
pragma(model="gpt2", max_tokens=2000, temperature=0.7)
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
