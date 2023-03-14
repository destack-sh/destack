from typing import Literal, Optional, cast

import sentry_sdk
import structlog
from graphql import DocumentNode as GraphQLDocumentNode
from graphql.language import DefinitionNode, OperationDefinitionNode
from strawberry.extensions import Extension

logger = structlog.get_logger(__name__)


def get_first_operation(
    definitions: list[DefinitionNode],
) -> Optional[OperationDefinitionNode]:
    definition = next(
        (
            cast(OperationDefinitionNode, node)
            for node in definitions
            if isinstance(node, OperationDefinitionNode)
        ),
        None,
    )
    return definition


def get_operation_name(operation_name: Optional[str], graphql_document: GraphQLDocumentNode) -> str:
    # If an operation_name has been specified then use that
    if operation_name:
        return operation_name

    definition = get_first_operation(graphql_document.definitions)
    if not definition:
        raise RuntimeError("Can't get GraphQL operation")

    if not definition.name:
        return "unnamed"

    return definition.name.value


graphql_operation_types = Literal["QUERY", "MUTATION", "SUBSCRIPTION"]


def get_operation_type(
    operation_name: Optional[str], graphql_document: GraphQLDocumentNode
) -> graphql_operation_types:
    definition: Optional[OperationDefinitionNode] = None

    # If no operation_name has been specified then use the first
    # OperationDefinitionNode
    if not operation_name:
        definition = get_first_operation(graphql_document.definitions)
    else:
        for d in graphql_document.definitions:
            d = cast(OperationDefinitionNode, d)
            if d.name and d.name.value == operation_name:
                definition = d
                break

    if not definition:
        raise RuntimeError("Can't get GraphQL operation type")

    return cast(graphql_operation_types, definition.operation.name)


class SentryPerformanceExtension(Extension):
    def on_request_start(self):
        self._transaction = sentry_sdk.start_transaction(
            op="api", custom_sampling_context={"strawberry_context": self.execution_context}
        )
        self._transaction.__enter__()

    def on_parsing_start(self):
        self._parsing_span = sentry_sdk.start_span(op="parsing", description="GraphQL parsing")
        self._parsing_span.__enter__()

    def on_parsing_end(self):
        self._parsing_span.__exit__(None, None, None)

    def on_validation_start(self):
        self._validation_span = sentry_sdk.start_span(
            op="validation", description="GraphQL validation"
        )
        self._validation_span.__enter__()

    def on_validation_end(self):
        self._validation_span.finish()
        self._validation_span.__exit__(None, None, None)

    def on_request_end(self):
        execution_context = self.execution_context

        if not execution_context.graphql_document:
            logger.warning(
                "execution_missing_graphql_document", execution_context=execution_context
            )
            self._transaction.name = "<unknown>"
            self._transaction.__exit__(None, None, None)
            return

        operation_name = get_operation_name(
            execution_context.operation_name, execution_context.graphql_document
        )
        operation_type = get_operation_type(
            execution_context.operation_name, execution_context.graphql_document
        )

        self._transaction.op = operation_type.lower()
        self._transaction.name = operation_name
        self._transaction.__exit__(None, None, None)
