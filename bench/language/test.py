from __future__ import annotations

from uuid import UUID, uuid4

from bench.language.basic import TextMention, TextSpan, parse_text_html, render_text_html
from bench.language.const import TriggerType
from bench.language.core import MNT, SessionContext, TypedNodeReference
from bench.utils.utils import IdentifierType, to_pyidentifier

MOCK_SESSION_CONTEXT = SessionContext(
    module_id=UUID("00000000-0000-0000-0000-000000000000"),
    project_id=UUID("00000000-0000-0000-0000-000000000000"),
    worker_node_id="test",
    worker_process_id="test",
    trigger_id=UUID("00000000-0000-0000-0000-000000000000"),
    trigger_type=TriggerType.API,
    first_run_id=UUID("00000000-0000-0000-0000-000000000000"),
)


def test_pyident():
    assert to_pyidentifier("MyStructIsCool", IdentifierType.TYPE) == "MyStructIsCool"
    assert to_pyidentifier("-test-ificate", IdentifierType.TYPE) == "TestIficate"
    assert to_pyidentifier("A Name", IdentifierType.VARIABLE) == "a_name"
    assert to_pyidentifier("A Name", IdentifierType.TYPE) == "AName"
    assert to_pyidentifier("my constant (2)", IdentifierType.CONSTANT) == "MyConstant2"


def test_parse_text():
    spans = [
        TextSpan(text="Hello "),
        TextMention.from_reference(TypedNodeReference(MNT.Field, uuid4())),
        TextSpan(text=", it's "),
        TextMention.from_reference(TypedNodeReference(MNT.Statement, uuid4()), path=".xyz"),
        TextSpan(text="!"),
    ]
    rendered = render_text_html(spans)
    parsed = parse_text_html(rendered)
    assert parsed == spans
