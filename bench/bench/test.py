from __future__ import annotations

from uuid import UUID

import pytest

from bench.bench import Code, Dataset, Field, Task, Type, TypeHint, TypeTag
from bench.bench.code_ import parse_code
from bench.bench.const import TypeFlag
from bench.bench.core import (
    LookupBy,
    Module,
    Session,
    SessionContext,
    SessionTracingLevel,
    StatementPath,
)
from bench.bench.issue import BenchError, IssueType
from bench.utils.utils import IdentifierType, to_pyidentifier


def test_extract_code_references():
    analysis = parse_code(
        """
import asyncio
await asyncio.sleep("ban")
for doc in some_documents:
    doc["name"] = doc.file.name
    doc["title"] = doc.file.name
    """
    )
    assert analysis.references == {
        "some_documents": StatementPath(".", "some_documents"),
    }
    assert analysis.is_async


def test_extract_code_references_shadowed():
    analysis = parse_code(
        """
import os
test.env = str(list(os.environ.keys()))
test.cwd = os.getcwd()
file_paths = []

def test(page):
    print(page)

# Walk through the directory and its subdirectories
for root, dirs, files in os.walk(test.cwd):
    for file in files:
        file_paths.append(os.path.join(root, file))
test.files = ",".join(file_paths[:50])
    """
    )
    assert analysis.references == {
        "test": StatementPath(".", "test"),
    }
    assert not analysis.is_async


def test_extract_code_references_imported():
    analysis = parse_code(
        """
from x.symbolx.lib.nlp import EntityType
from x.notion.sdk import Client as NotionClient
from .x.cooking import Recipe, Ingredient, notion_recipes
print(bananas)
""",
    )
    assert analysis.references == {
        "EntityType": StatementPath("symbolx.lib.nlp", "EntityType"),
        "NotionClient": StatementPath("notion.sdk", "Client"),
        "Recipe": StatementPath(".cooking", "Recipe"),
        "Ingredient": StatementPath(".cooking", "Ingredient"),
        "notion_recipes": StatementPath(".cooking", "notion_recipes"),
        "bananas": StatementPath(".", "bananas"),
    }
    assert not analysis.is_async


MOCK_SESSION_CONTEXT = SessionContext(
    project_id=UUID("00000000-0000-0000-0000-000000000000"),
    module_id=UUID("00000000-0000-0000-0000-000000000000"),
    worker_id=UUID("00000000-0000-0000-0000-000000000000"),
    trigger_id=UUID("00000000-0000-0000-0000-000000000000"),
    root_id=UUID("00000000-0000-0000-0000-000000000000"),
    trigger_type=None,
    tracing_level=SessionTracingLevel.ALL,
)


def test_type_union_with():
    module = Module(name="test")
    with Session(module, ctx=MOCK_SESSION_CONTEXT).sync():
        resource = Type(name="Resource", tag=TypeTag.STRUCT).add_field(
            Field(name="name", tag=TypeTag.STRING)
        )
        connection = (
            Type(name="Connection", tag=TypeTag.STRUCT)
            .extend_type(resource)
            .add_field(Field(name="access_token", tag=TypeTag.STRING))
        )
        oauth_connection = (
            Type(name="OAuthConnection", tag=TypeTag.STRUCT)
            .extend_type(connection)
            .add_field(
                Field(
                    name="refresh_token",
                    flags=TypeFlag.IsNullable,
                    tag=TypeTag.STRING,
                )
            )
        )
        github_thing = Type(
            name="GithubThing",
            tag=TypeTag.STRUCT,
            fields=[Field(name="github_id", tag=TypeTag.STRING, hint=TypeHint.UUID)],
        )
        github_connection = (
            Type(name="GithubConnection", tag=TypeTag.STRUCT)
            .extend_type(oauth_connection, github_thing)
            .add_field(
                Field(name="username", tag=TypeTag.STRING),
                Field(name="email", tag=TypeTag.STRING),
            )
        )

    # members should include inherited members
    github_connection_names = (i.name for i in github_connection.resolved_fields)
    assert set(github_connection_names) == {
        "name",
        "username",
        "email",
        "github_id",
        "refresh_token",
        "access_token",
    }


def test_recursive_union_fail():
    module = Module(name="test")
    with Session(module, ctx=MOCK_SESSION_CONTEXT).sync():
        with pytest.raises(BenchError) as e:
            type_a = Type(name="A", tag=TypeTag.STRUCT)
            type_a.extend_type(type_a)
    assert e.value.issue.type == IssueType.CIRCULAR_UNION


def test_pyident():
    assert to_pyidentifier("MyStructIsCool", IdentifierType.TYPE) == "MyStructIsCool"
    assert to_pyidentifier("-test-ificate", IdentifierType.TYPE) == "TestIficate"
    assert to_pyidentifier("A Name", IdentifierType.VARIABLE) == "a_name"
    assert to_pyidentifier("A Name", IdentifierType.TYPE) == "AName"
    assert to_pyidentifier("my constant (2)", IdentifierType.CONSTANT) == "MY_CONSTANT_2"


def test_nested_resolve():
    module = Module(name="test.lib")
    file = module.create_file("test-file")
    struct = Type(name="MyStruct", tag=TypeTag.STRUCT)
    task = Task(name="Organize goats")
    dataset = Dataset(name="dataset")
    code = Code(name="load")
    dataset.append_child(code)
    file.append(struct, task, dataset)
    module.index()
    module.interp()

    # absolute with module name
    assert module.lookup("test.lib.test-file.MyStruct") == struct
    assert module.lookup("test.lib.test_file.MyStruct", LookupBy.PyIdent) == struct
    assert module.lookup("test.lib.test-file.Organize goats") == task
    assert module.lookup("test.lib.test_file.organize_goats", LookupBy.PyIdent) == task
    assert module.lookup("test.lib.test-file.dataset") == dataset
    assert module.lookup("test.lib.test-file.dataset.load") == code

    # absolute local
    assert module.lookup(".test-file.dataset") == dataset
    assert module.lookup(".test-file.dataset.load") == code

    # relative
    assert code.lookup("dataset") == dataset
    assert code.lookup("organize_goats", LookupBy.PyIdent) == task

    # self
    assert module.lookup(struct.path, LookupBy.PyIdent) == struct
    assert module.lookup(task.path, LookupBy.PyIdent) == task
    assert module.lookup(dataset.path, LookupBy.PyIdent) == dataset
    assert module.lookup(code.path, LookupBy.PyIdent) == code
