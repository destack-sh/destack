from __future__ import annotations

from uuid import uuid4

from bench.language.const import MNT, TypedNodeReference
from bench.language.text import (
    TextMention,
    TextPlain,
    parse_text_html,
    patch_text_html,
    render_text_html,
)
from bench.utils.utils import IdentifierType, to_pyidentifier


def test_pyident():
    assert to_pyidentifier("MyStructIsCool", IdentifierType.TYPE) == "MyStructIsCool"
    assert to_pyidentifier("-test-ificate", IdentifierType.TYPE) == "TestIficate"
    assert to_pyidentifier("A Name", IdentifierType.VARIABLE) == "a_name"
    assert to_pyidentifier("A Name", IdentifierType.TYPE) == "AName"
    assert to_pyidentifier("my constant (2)", IdentifierType.CONSTANT) == "MyConstant2"


def test_parse_text():
    spans = [
        TextPlain(text="Hello "),
        TextMention.from_reference(TypedNodeReference(MNT.FIELD, uuid4())),
        TextPlain(text=", it's "),
        TextMention.from_reference(TypedNodeReference(MNT.STATEMENT, uuid4()), path=".xyz"),
        TextPlain(text="!"),
    ]
    rendered = render_text_html(spans)
    parsed = parse_text_html(rendered)
    assert parsed == spans


def test_patch_text():
    statement_ck = uuid4()
    spans = [
        TextPlain(text="Hello "),
        TextMention.from_reference(TypedNodeReference(MNT.FIELD, uuid4())),
        TextPlain(text=", it's "),
        TextMention.from_reference(TypedNodeReference(MNT.STATEMENT, statement_ck), path=".xyz"),
        TextPlain(text="!"),
    ]
    new_statement_ck = uuid4()
    patched_text = patch_text_html(render_text_html(spans), {statement_ck: new_statement_ck})
    patched_spans = parse_text_html(patched_text)
    assert patched_spans[3].reference_ck == new_statement_ck
