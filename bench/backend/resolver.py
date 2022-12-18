from __future__ import annotations

import typing
from typing import Any

from django.db.models import QuerySet

from bench.backend.types import (
    ResolvedCode,
    ResolvedDataset,
    ResolvedModel,
    ResolvedParameter,
    ResolvedSymbol,
    Value,
)
from bench.models import (
    Argument,
    Code,
    Dataset,
    Model,
    ModelInferenceSettings,
    ParameterType,
    Statement,
    SymbolContent,
    SymbolType,
)
from bench.utils.record import RecordList


class Resolver:
    """
    Server-side resolver to fetch all recursive arguments and contents.
    """

    async def resolve_model(
        self, model: Model, settings: typing.Optional[ModelInferenceSettings]
    ) -> ResolvedModel:
        return ResolvedModel(
            statement_id=model.definition.id,
            content_id=model.id,
            name=model.definition.name,
            type=SymbolType.MODEL,
            settings=settings,
            default_settings=model.default_settings,
            provider=model.provider,
            external_name=model.external_name,
        )

    async def resolve_dataset(self, dataset: Dataset) -> ResolvedDataset:
        # TODO @Performance: do not load all records when resolving dataset arguments
        #  All functions are executed async, but dataset access is neater if it's synchronous.
        #  So we pre-load everything and wrap it in a synchronous wrapper.
        records = []
        async for record in dataset:
            records.append(record)
        batch = RecordList(records)
        return ResolvedDataset(
            statement_id=dataset.definition.id,
            content_id=dataset.id,
            name=dataset.definition.name,
            type=SymbolType.DATASET,
            schema=dataset.schema_,
            records=batch,
        )

    async def resolve_code(self, code: Code) -> ResolvedCode:
        parameters = await self._get_code_parameters(code)
        arguments = await self.resolve_arguments(code)
        return ResolvedCode(
            statement_id=code.definition.id,
            content_id=code.id,
            name=code.definition.name,
            type=SymbolType.CODE,
            input_schema=code.input_schema,
            output_schema=code.output_schema,
            code_text=code.code,
            code_function_name=code.code_function_name,
            builtin_id=code.builtin_id,
            parameters=parameters,
            arguments=arguments,
        )

    async def _get_code_parameters(self, code: Code) -> dict[str, ResolvedParameter]:
        parameters = {}
        async for parameter in code.parameters.all():
            parameters[parameter.name] = ResolvedParameter(name=parameter.name, type=parameter.type)
        return parameters

    async def resolve_arguments(self, code: Code) -> dict[str, ResolvedSymbol | Value]:
        bound_arguments: QuerySet[Argument] = code.arguments.all().select_related(
            "reference",
            "reference__model",
            "reference__model__default_settings",
            "reference__dataset",
            "reference__code",
        )
        bound_arguments_resolved: dict[str, Any] = {}
        async for argument in bound_arguments:
            if argument.type == ParameterType.VALUE:
                bound_arguments_resolved[argument.name] = argument.value
                continue
            if argument.reference is None:
                raise ValueError(f"argument {argument} has no statement reference")
            # resolve statement reference
            bound_arguments_resolved[argument.name] = await self.resolve_argument(
                argument.reference
            )
        return bound_arguments_resolved

    async def resolve_argument(
        self, value: Value | Statement | SymbolContent
    ) -> ResolvedSymbol | Value:
        if isinstance(value, SymbolContent):
            value = value.definition
        if isinstance(value, Statement):
            if value.type == SymbolType.MODEL:
                return await self.resolve_model(value.model_, settings=None)
            elif value.type == SymbolType.DATASET:
                return await self.resolve_dataset(value.dataset_)
            elif value.type == SymbolType.CODE:
                return await self.resolve_code(value.code_)
            else:
                raise ValueError(f"unexpected argument type: {value}")
        else:
            return value
