from __future__ import annotations

import ast
from inspect import cleandoc
from typing import Any

import pytest

from bench.language import Code, CodeKind
from bench.runtime.compiler import (
    CodeAnalysisVisitor,
    CodeDefinition,
    CodeDefinitionKind,
    CodeImport,
    compile_code,
    desugar_code,
)

# NOTE: some of the analysis logic was adapted from marimo (Apache 2 licensed, also see analysis)
#  see https://github.com/marimo-team/marimo/blob/fec7d780488ab1478984468598d00d283e8c1c9d/tests/_ast/test_compiler.py


class _TestVisitorHandle:
    """Convenience wrapper around our analyzing visitor for testing."""

    def __init__(
        self,
        kind: CodeKind = CodeKind.SCRIPT,
        visitor: CodeAnalysisVisitor | None = None,
    ):
        self.visitor = visitor or CodeAnalysisVisitor(kind=kind)

    @property
    def definitions(self):
        return self.visitor.definitions

    @property
    def defs(self) -> set[str]:
        """Get all global defs."""
        return set(self.visitor._block_stack[0].definitions.keys())

    @property
    def refs(self) -> set[str]:
        """Get all global refs."""
        return set(self.visitor._references.keys())

    def visit(self, mod: ast.Module):
        self.visitor.visit(mod)


def test_assign_simple():
    expr = "x = 0"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"x"}
    assert v.refs == set()
    assert v.definitions == {"x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE)}


def test_multiple_assign():
    expr = "x = y = 0"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"x", "y"}
    assert v.refs == set()
    assert v.definitions == {
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "y": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_assign_multiple_statements():
    code = "x = 0\ny = 0"
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"x", "y"}
    assert v.refs == set()
    assert v.definitions == {
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "y": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_assign_attr():
    expr = "x.a = 0"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == set()
    assert v.refs == {"x"}
    assert not v.definitions


def test_read_attr():
    expr = "x.a"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == set()
    assert v.refs == {"x"}
    assert not v.definitions


def test_read_attr_of_defined_variable():
    expr = "x = 0; x.a"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"x"}
    assert v.refs == set()
    assert v.definitions == {"x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE)}


def test_assign_nested_attr():
    expr = "x.a.b = 0"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == set()
    assert v.refs == {"x"}
    assert not v.definitions


def test_read_nested_attr():
    expr = "x.a.b"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == set()
    assert v.refs == {"x"}
    assert not v.definitions


def test_assign_same_name():
    expr = "x = x"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"x"}
    assert v.refs == {"x"}
    assert v.definitions == {
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references={"x"})
    }

    expr = "x=1; x = x"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"x"}
    assert v.refs == set()
    assert v.definitions == {
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references={"x"})
    }

    expr = "(x := x)"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"x"}
    assert v.refs == {"x"}
    assert v.definitions == {
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references={"x"})
    }

    expr = "x += x"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"x"}
    assert v.refs == {"x"}
    assert v.definitions == {
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references={"x"})
    }

    expr = "def f(): x = x; return x"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"f"}
    assert v.refs == {"x"}
    assert v.definitions == {
        "f": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"x"})
    }

    expr = "class F(): x = x"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"F"}
    assert v.refs == {"x"}
    assert v.definitions == {"F": CodeDefinition(kind=CodeDefinitionKind.CLASS, references={"x"})}

    expr = "{x: x}"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == set()
    assert v.refs == {"x"}
    assert not v.definitions


def test_load_attr():
    expr = "x.a"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == set()
    assert v.refs == {"x"}
    assert not v.definitions


def test_structured_assignment():
    expr = "(a, (b, c, (d, e)), (f, g), h) = 0"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    names = {"a", "b", "c", "d", "e", "f", "g", "h"}
    assert v.defs == names
    assert v.refs == set()
    assert v.definitions == {
        "a": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "b": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "c": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "d": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "e": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "f": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "g": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "h": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_starred_assignment():
    expr = "a, *b = 0"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"a", "b"}
    assert v.refs == set()
    assert v.definitions == {
        "a": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "b": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_scope_does_not_leak():
    code = "\n".join(
        [
            "def foo():",
            "  z = 0",
            "z",
        ]
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"foo"}
    assert v.refs == set("z")
    assert v.definitions == {
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION),
    }


def test_nested_comprehensions():
    code = "\n".join(
        [
            "[(i, j) for i in range(10) for j in range(i)]",
            "{(i, j) for i in range(10) for j in range(i)}",
            "{i: j for i in range(10) for j in range(i)}",
        ]
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == set()
    assert v.refs == {"range"}
    assert not v.definitions


def test_walrus_leaks_to_global_in_comprehension():
    code = "\n".join(
        [
            "def foo(): a",
            "[(a := (i, (b := j))) for i in range(10) for j in range(i)]",
            "{(c := (i, j)) for i in range(10) for j in range(i)}",
            "{i: (d := j) for i in range(10) for j in range(i)}",
        ]
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"a", "b", "c", "d", "foo"}
    # "a" should not be a ref!
    assert v.refs == {"range"}
    assert v.definitions == {
        "a": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "b": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "c": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "d": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"a"}),
    }


def test_nested_walrus_leaks_to_global_in_comprehension():
    code = "[[(a := (i, (b := j))) for j in range(i)] for i in range(10)]"
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"a", "b"}
    assert v.refs == {"range"}
    assert v.definitions == {
        "a": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "b": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_pep572_walrus_comprehension_examples():
    code = "[(x, y, x/y) for x in input_data if (y := f(x)) > 0]"
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"y"}
    assert v.refs == {"input_data", "f"}
    assert v.definitions == {
        "y": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }

    code = "[[y := f(x), x/y] for x in range(5)]"
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"y"}
    assert v.refs == {"f", "range"}
    assert v.definitions == {
        "y": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_walrus_in_comp_in_fn_block_does_not_leak_to_global():
    code = """\
def f():
    [(x := 0) for i in range(5)]
    y = x + 1
"""
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"f"}  # x should _not_ leak to global scope
    assert v.refs == {"range"}  # x should leak to f's scope
    assert v.definitions == {
        "f": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"range"}),
    }


def test_assignments_in_multiple_scopes():
    code = """\
a = 0
def foo():
    b = 0
    c = 0
    def bar():
        d = 0
e = 0
"""
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"a", "e", "foo"}
    assert v.refs == set()
    assert v.definitions == {
        "a": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "e": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION),
    }


def test_function_with_args():
    code = cleandoc(
        """
        def foo(a: 'annotation', b=1, c=2, *d, e, f=3, **g):
          y = a + z
          return y
        """
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"foo"}
    assert v.refs == set("z")
    assert v.definitions == {
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"z"}),
    }


def test_function_with_defaults():
    code = cleandoc(
        """
        def foo(x=y, y=x, z=a):
          pass
        """
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"foo"}
    assert v.refs == {"x", "y", "a"}
    # TODO: Are these required refs?
    assert v.definitions == {
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"x", "y", "a"}),
    }


def test_async_function_def():
    code = cleandoc(
        """
        async def foo(a):
          y = a + z

        x  = 0
        """
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"foo", "x"}
    assert v.refs == set("z")
    assert v.definitions == {
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"z"}),
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_global_def():
    code = cleandoc(
        """
        def foo(a):
          global x
          x = 0
        """
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"foo", "x"}
    assert v.refs == set()
    assert v.definitions == {
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"x"}),
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_global_ref():
    code = cleandoc(
        """
        def foo(a):
          global x
          print(x)
        """
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"foo"}
    assert v.refs == {"x", "print"}
    assert v.definitions == {
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"x", "print"}),
    }


def test_nested_local_def_and_global_ref():
    code = cleandoc(
        """
        def foo(a):
          global x
          def bar():
            x = 10
          print(x)
        """
    )
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"foo"}
    assert v.refs == {"x", "print"}
    assert v.definitions == {
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"x", "print"}),
    }


def test_call_ref():
    code = "foo()"
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == set()
    assert v.refs == {"foo"}
    assert not v.definitions


def test_call_defined():
    # fmt: off
    code = "\n".join([
        "def foo():",
        "  pass",
        "foo()"
    ])
    # fmt: on
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert v.defs == {"foo"}
    assert v.refs == set()
    assert v.definitions == {
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION),
    }


def test_mutation_generates_def():
    code = "x += 5"
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"x"}
    assert v.refs == set()
    assert v.definitions == {
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_captured_variables():
    code = cleandoc(
        """
        x = 0

        def f():
            x
        """
    )
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"f", "x"}
    assert v.refs == set()

    code = cleandoc(
        """
        def f():
            x

        x = 0
        """
    )
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"f", "x"}
    assert v.refs == set()

    code = cleandoc(
        """
        def f():
            def g():
                x

        x = 0
        """
    )
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"f", "x"}
    assert v.refs == set()

    code = cleandoc(
        """
        def f():
            def g():
                x
            x = 0
        """
    )
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"f"}
    assert v.refs == set()

    code = cleandoc(
        """
        def f():
            def g():
                x

        def h():
            x = 1
        """
    )
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"f", "h"}
    assert v.refs == {"x"}


def test_matchas():
    code = cleandoc(
        """
        match value:
            case [a]:
                ...
            case (b, c):
                ...
            case d:
                ...
            case e as f:
                ...
        """
    )
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"a", "b", "c", "d", "e", "f"}
    assert v.refs == {"value"}
    assert v.definitions == {
        "a": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "b": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "c": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "d": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "e": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "f": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_matchstar():
    code = cleandoc(
        """
        match value:
            case [1, 2, *rest]:
                ...
            case [*_]:
                ...
        """
    )
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"rest"}
    assert v.refs == {"value"}
    assert v.definitions == {"rest": CodeDefinition(kind=CodeDefinitionKind.VARIABLE)}


def test_matchmapping():
    code = cleandoc(
        """
        match value:
            case {1: _, 2: _, **a}:
                ...
            case {**b}:
                ...
            case {1: _}:
                ...
        """
    )
    mod = ast.parse(code)
    v = _TestVisitorHandle()
    v.visit(mod)
    assert v.defs == {"a", "b"}
    assert v.refs == {"value"}
    assert v.definitions == {
        "a": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "b": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
    }


def test_import_nested():
    expr = "import a.b.c"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"a"}
    assert v.refs == set()
    assert v.definitions["a"] == CodeDefinition(
        kind=CodeDefinitionKind.IMPORT, imprt=CodeImport(module="a.b.c", fully_qualified_name=None)
    )


def test_import_as():
    expr = "import a.b.c as d"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"d"}
    assert v.refs == set()
    assert v.definitions["d"] == CodeDefinition(
        kind=CodeDefinitionKind.IMPORT, imprt=CodeImport(module="a.b.c", fully_qualified_name=None)
    )


def test_import_multiple():
    expr = "import a.b.c, d"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"a", "d"}
    assert v.refs == set()
    assert v.definitions["a"] == CodeDefinition(
        kind=CodeDefinitionKind.IMPORT, imprt=CodeImport(module="a.b.c", fully_qualified_name=None)
    )
    assert v.definitions["d"] == CodeDefinition(
        kind=CodeDefinitionKind.IMPORT, imprt=CodeImport(module="d", fully_qualified_name=None)
    )


def test_from_import():
    expr = "from a.b.c import d"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"d"}
    assert v.refs == set()
    assert v.definitions["d"] == CodeDefinition(
        kind=CodeDefinitionKind.IMPORT,
        imprt=CodeImport(
            module="a.b.c",
            fully_qualified_name="a.b.c.d",
            original_name="d",
            as_name="d",
            relative_level=0,
        ),
    )


def test_relative_from_import():
    expr = "from ..a.b.c import d as e"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"e"}
    assert v.refs == set()
    assert v.definitions["e"] == CodeDefinition(
        kind=CodeDefinitionKind.IMPORT,
        imprt=CodeImport(
            module="a.b.c",
            fully_qualified_name="a.b.c.d",
            original_name="d",
            as_name="e",
            relative_level=2,
        ),
    )


def test_from_import_star():
    expr = "from a.b.c import *"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    with pytest.raises(SyntaxError) as e:
        v.visit(mod)
    assert "`import *` is not allowed" in str(e)
    assert v.defs == set()
    assert v.refs == set()
    assert not v.definitions


def test_try_block():
    code = """\
f = 2
try:
  v = 1 / 0
except TypeError as e:	
  err = e
  e = 0
  x = out_of_scope
  print(f'caught {type(e)} with nested {e.exceptions}')
except OSError as f:
  err2 = f
  try:
    y = 1 / 0
  except ZeroDivisionError as g:
    err3 = g
else:
  w = 1
finally:
  z = 3
"""
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    # T should not be among the refs or defs
    assert "out_of_scope" in v.refs
    assert "e" not in v.refs
    assert "f" not in v.refs
    assert "g" not in v.refs
    assert v.defs == {"f", "v", "w", "x", "y", "z", "err", "err2", "err3"}


def test_try_star_block():
    code = """\
try:
    raise ExceptionGroup('eg', [ValueError(1), TypeError(2), OSError(3)])
except* TypeError as e:
    print(f'caught {type(e)} with nested {e.exceptions}')
except* OSError as f:
    print(f'caught {type(f)} with nested {f.exceptions}')
else:
    print('Type and Os not raised')
finally:
    print('finally')
"""
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    # T should not be among the refs or defs
    assert v.defs == set()
    assert "e" not in v.refs
    assert "f" not in v.refs


def test_type_alias_scoped():
    expr = "type alias[T] = list[T]"
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    # T should not be among the refs or defs
    assert v.defs == {"alias"}
    assert v.refs == {"list"}


def test_type_var_generic_class():
    expr = cleandoc(
        """
    class A[T]:
        def hello(self, x: T) -> T:
            T
    """
    )
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    # T should not be among the refs or defs
    assert v.defs == {"A"}
    assert v.refs == set()


def test_type_var_generic_function():
    expr = cleandoc(
        """
        def test[U](u: U) -> U:
            return u
        """
    )
    v = _TestVisitorHandle()
    mod = ast.parse(expr)
    v.visit(mod)
    assert v.defs == {"test"}
    # U should not be a ref
    assert v.refs == set()


def test_private_ref_requirement_caught():
    code = """\
x = 1
_x = 1
def foo():
    z = _x + x + X
"""
    v = _TestVisitorHandle()
    mod = ast.parse(code)
    v.visit(mod)
    assert len(v.defs & {"foo", "x"}) == 2
    assert len(v.defs - {"foo", "x"}) == 1
    (private,) = v.defs - {"foo", "x"}
    assert private.startswith("_")
    assert private.endswith("_x")
    assert v.refs == {"X"}
    assert v.definitions == {
        private: CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
        "foo": CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references={"X", "x", private}),
    }


def test_compile_code_snippet():
    code = Code.from_string("""\
x = 1 + y + CONST
_y = x + 1
_y
""")
    compiled = compile_code("anon", code.to_string(), CodeKind.SNIPPET, {"CONST": 0})
    assert compiled.code == code.to_string()
    assert compiled.transformed_code == compiled.code  # no transformation
    assert set(compiled.references.keys()) == {"y"}  # exclude global refs


def test_compile_code_script():
    code = Code.from_string("""\
x = y + 1
def a():
    pass
""")
    compiled = compile_code("anon", code.to_string(), CodeKind.SCRIPT, {})
    assert compiled.code == code.to_string()
    assert compiled.transformed_code == compiled.code  # no transformation
    assert set(compiled.references.keys()) == {"y"}
    assert compiled.definitions == {
        "x": CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references={"y"}),
        "a": CodeDefinition(kind=CodeDefinitionKind.FUNCTION),
    }


def test_compile_code_function():
    code = Code.from_string("""\
x = 1
return Input1 + y + 1
""")
    compiled = compile_code("anon", code.to_string(), CodeKind.FUNCTION, {})
    assert compiled.code == code.to_string()
    assert set(compiled.references.keys()) == {"Input1", "y"}

    # run it to check function definition
    glbls: dict[str, Any] = {"Input1": 1, "y": 1}
    assert compiled.body_co, f"no code object for {compiled!r}"
    exec(compiled.body_co, glbls)
    func = glbls["_code_anon"]
    assert func() == 3


def test_compile_code_function_with_multiple_returns():
    code = Code.from_string("""\
if not x:
    return
if a or c:
    return 1, 2, 3
elif c > 5:
    pass
else:
    while not z:
        _x = boomify(a)
    return (
        1,
        2,
        3,
    )
""")
    compiled = compile_code("anon", code.to_string(), CodeKind.FUNCTION, {})
    assert compiled.code == code.to_string()
    assert set(compiled.references.keys()) == {"x", "a", "c", "z", "boomify"}
    assert compiled.body_co, f"no code object for {compiled!r}"


@pytest.mark.skip(":Incomplete")
def test_desugar_code_paths():
    code = """\
my_node = >Node/Something
my_grandchild = $ThisChild/ThatGrandchild
other_node = $"/OtherNode/Child With Space"
if ^unique_node/child:
    for child in (^unique_node/child).fields:
        print(child)
print("unrelated/^path/dont/@replace/me")
\"\"\"
"""
    desugared, _ = desugar_code(code)
    assert (
        desugared
        == """\
my_node = get_node("Node/Something")
my_grandchild = get_node("ThisChild/ThatGrandchild")
other_node = get_node("/OtherNode/Child With Space")
if get_node("^unique_node/child"):
    for child in get_node("^unique_node/child").fields:
        print(child)
print("unrelated/^path/dont/@replace/me")
"""
    )
