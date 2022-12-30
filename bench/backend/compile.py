from __future__ import annotations

import enum

import structlog

from bench.models import Code, Model, Organization, Project, SymbolType

logger = structlog.get_logger(__name__)


def get_stdlib_model(owner_slug: str, model_name: str) -> Model:
    """
    Gets the backend model from a stdlib library where backend=owner/model
    """
    organization = Organization.objects.get(slug=owner_slug)
    stdlib: Project = organization.projects.get(slug="stdlib")
    return stdlib.head_.statement(file=None, name=model_name, symbol_type=SymbolType.MODEL).model_


def get_stdlib_code(owner_slug: str, code_name) -> Code:
    """
    Gets the code from a stdlib where path=owner/code
    """
    organization = Organization.objects.get(slug=owner_slug)
    stdlib: Project = organization.projects.get(slug="stdlib")
    return stdlib.head_.statement(file=None, name=code_name, symbol_type=SymbolType.CODE).code_


class ExpectationStatementType(enum.Enum):
    """
    The type of expectation defines its semantics.
    """

    GENERATE = "generate"
    TRANSFORM = "transform"
    VERIFY = "verify"
