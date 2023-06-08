from __future__ import annotations

import pytest

from bench.language import TypeHint, TypeTag
from bench.language.const import TypeFlag
from bench.language.issue import IssueType, LanguageError
from bench.language.parse import parse_code
from bench.language.type import Field, StatementPath, Type


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
from x.symbolx.std.nlp import EntityType
from x.notion.sdk import Client as NotionClient
from .x.cooking import Recipe, Ingredient, notion_recipes
print(bananas)
""",
    )
    assert analysis.references == {
        "EntityType": StatementPath("symbolx.std.nlp", "EntityType"),
        "NotionClient": StatementPath("notion.sdk", "Client"),
        "Recipe": StatementPath(".cooking", "Recipe"),
        "Ingredient": StatementPath(".cooking", "Ingredient"),
        "notion_recipes": StatementPath(".cooking", "notion_recipes"),
        "bananas": StatementPath(".", "bananas"),
    }
    assert not analysis.is_async


def test_type_union_with():
    resource = Type(name="Resource", tag=TypeTag.STRUCT).append(
        Field(name="name", tag=TypeTag.STRING)
    )
    connection = (
        Type(name="Connection", tag=TypeTag.STRUCT)
        .extend(resource)
        .append(Field(name="access_token", tag=TypeTag.STRING))
    )
    oauth_connection = (
        Type(name="OAuthConnection", tag=TypeTag.STRUCT)
        .extend(connection)
        .append(
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
        .extend(oauth_connection)
        .extend(github_thing)
        .append(
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
    with pytest.raises(LanguageError) as e:
        type_a = Type(name="A", tag=TypeTag.STRUCT)
        type_b = Type(name="B", tag=TypeTag.STRUCT)
        type_a.extend(type_b)
        type_b.extend(type_a)
    assert e.type == IssueType.CIRCULAR_UNION
