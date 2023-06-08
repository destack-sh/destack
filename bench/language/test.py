from __future__ import annotations

import functools
from typing import Callable

from bench.language import TypeTag
from bench.language.interp import Error, ErrorCollector, IssueType, parse_code
from bench.language.type import Code, StatementPath, Type, TypeFlag


def _raise_if_error(error: Error, test: Callable):
    if test(error):
        raise error


def _raise_if(test: Callable):
    return functools.partial(_raise_if_error, test=test)


def _raise_if_not_external():
    return _raise_if(lambda e: e.type != IssueType.EXTERNAL_LOOKUP_FAILED)


def test_absolute_references():
    module, idx = parse_string(
        """
--- a ---
type Apple:
- name: string

type AppleTree:
- apples: [Apple]

code graft :: (tree: .a.AppleTree, apple: Apple) -> (tree: AppleTree):
```python
num_branches: int = 0 # not used (for testing)
return tree + apple
```

--- b ---

type FruitBasket:
- apples: [.a.Apple]
"""
    )
    type_graft = idx.symbol(".a:graft", Code)
    type_fruit_basket = idx.symbol(".b:FruitBasket", Type)
    # apple inside graft should be the same apple as the one in the fruit basket
    type_graft_apple = type_graft.type["apple"]
    type_fruit_basket_apple = type_fruit_basket["apples"]
    assert type_graft_apple.reference.id == type_fruit_basket_apple.reference.id


def test_resolve_nested_aliased_type():
    module, idx = parse_string(
        """
--- test ---
type RealString = string
type MyString = RealString
type EntityType = MyString

type Entity:
- name: string
- 'type': EntityType
"""
    )

    type_entity_type = idx.symbol(".test:EntityType", Type)
    assert type_entity_type.tag == TypeTag.STRING

    type_entity = idx.symbol(".test:Entity", Type)
    assert type_entity["type"].tag == TypeTag.STRING


def test_resolve_circular_type():
    with Session():
        entity = Type(
            "Entity",
            {
                "name": TypeTag.STRING,
                "first_event": (TypeFlag.IsNullable, TypeTag.REFERENCE),
            },
        )
        Type(
            "Event",
            {
                "summary": TypeTag.STRING,
                "entities": (TypeFlag.IsArray, entity),
            },
        )

    module, idx = parse_string(
        """
--- test ---
type Entity:
- name: string
- first_event: Event?

type Event:
- summary: string
- entities: [Entity]
"""
    )

    type_event = idx.symbol(".test:Event", Type)
    assert type_event["entities"].flags & TypeFlag.IsArray

    type_entity = idx.symbol(".test:Entity", Type)
    assert type_entity["first_event"].reference.id == type_event.id


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
    bench = """
--- test ---
type Resource:
- name: string

type Connection:
& Resource
- access_token: string

type OAuthConnection:
& Connection
- refresh_token: string?

type GithubConnection:
& OAuthConnection
& GithubThing
- username: string
- email: string

type GithubRepository:
& GithubThing
- owner: string
- name: string

type GithubThing:
- github_id: string

dataset connections :: (name: string?) & GithubConnection:
```jsonl
{}
```

dataset repositories :: GithubRepository:
```jsonl
{}
```

code test_connection :: (repository: GithubRepository) & GithubConnection -> (success: boolean):
```python
pass
```
""".strip()
    module, idx = parse_string(bench)
    # members should include inherited members
    github_connection_names = (i.name for i in idx.symbol(".test:GithubConnection", Type).fields)
    assert set(github_connection_names) == {
        "name",
        "username",
        "email",
        "github_id",
        "refresh_token",
        "access_token",
    }

    # rendered should match
    reconstructed = render(module.files)
    assert reconstructed == bench


def test_recursive_union_fail():
    collector = ErrorCollector()
    module, idx = parse_string(
        """
--- test ---
type A:
& B

--- test ---
type B:
& A
""",
        on_error=collector,
    )
    assert len(collector.errors) == 1
    assert collector.errors[0].type == IssueType.CIRCULAR_UNION
