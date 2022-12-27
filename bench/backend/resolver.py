from __future__ import annotations

import typing
from typing import Any

from django.db.models import QuerySet

from bench.backend.types import Value
from bench.language.types import Code, Dataset, Model, Statement, SymbolContent
from bench.models import (
    Code,
    Dataset,
    Model,
    ModelInferenceSettings,
    Statement,
    SymbolContent,
    SymbolType,
)
from bench.utils.record import RecordList
from bench.utils.schema import SchemaElement


class Resolver:
    """
    Server-side resolver to remotely read and write files and statements.
    Keeps track of revisions used for automatic caching and re-computation.
    """

    def read_statement(self, statement: Statement) -> Statement:
        raise NotImplementedError

    def write_statement(self, statement: Statement) -> Statement:
        raise NotImplementedError

    def read_model(self, model: Model, settings: typing.Optional[ModelInferenceSettings]) -> Model:
        return Model(
            statement_id=model.definition.id,
            content_id=model.id,
            name=model.definition.name,
            type=SymbolType.MODEL,
            settings=settings,
            default_settings=model.default_settings,
            provider=model.provider,
            external_name=model.external_name,
        )

    def read_dataset(self, dataset: Dataset) -> Dataset:
        # TODO @Performance: do not load all records when resolving dataset arguments
        #  All functions are executed async, but dataset access is neater if it's synchronous.
        #  So we pre-load everything and wrap it in a synchronous wrapper.
        records = list(dataset)
        batch = RecordList(records)
        return Dataset(
            statement_id=dataset.definition.id,
            content_id=dataset.id,
            name=dataset.definition.name,
            type=SymbolType.DATASET,
            schema=dataset.schema_.element,
            records=batch,
        )

    def read_code(self, code: Code) -> Code:
        parameters = self._get_code_parameters(code)
        arguments = self.read_arguments(code)
        schema: SchemaElement = code.schema_.element
        return Code(
            statement_id=code.definition.id,
            content_id=code.id,
            name=code.definition.name,
            type=SymbolType.CODE,
            input_schema=schema.input_,
            output_schema=schema.output_,
            code_text=code.code,
            code_function_name=code.code_function_name,
            builtin_id=code.builtin_id,
            parameters=parameters,
            arguments=arguments,
        )

    def read_arguments(self, code: Code) -> dict[str, SymbolContent | Value]:
        bound_arguments: QuerySet[Statement] = code.arguments.all().select_related(
            "reference",
            "reference__model",
            "reference__model__default_settings",
            "reference__dataset",
            "reference__code",
        )
        bound_arguments_resolved: dict[str, Any] = {}
        for argument in bound_arguments:
            if argument.reference is None:
                raise ValueError(f"argument {argument} has no statement reference")
            # resolve statement reference
            bound_arguments_resolved[argument.name] = self.read_argument(argument.reference)
        return bound_arguments_resolved

    def read_argument(self, value: Statement | SymbolContent) -> SymbolContent | Value:
        if isinstance(value, SymbolContent):
            value = value.definition
        if isinstance(value, Statement):
            if value.type == SymbolType.MODEL:
                return self.read_model(value.model_, settings=None)
            elif value.type == SymbolType.DATASET:
                return self.read_dataset(value.dataset_)
            elif value.type == SymbolType.CODE:
                return self.read_code(value.code_)
            else:
                raise ValueError(f"unexpected argument type: {value}")
        else:
            return value
