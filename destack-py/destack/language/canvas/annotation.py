from destack.language.core import Node, NodeType, Text, builtin_node, property_
from destack.pb2 import AnnotationShapeData

from ..view import ContainerView
from .shape import IsShape

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ANNOTATION_SHAPE, pretend_frozen=True)
class AnnotationShape(ContainerView, IsShape, Node[AnnotationShapeData]):
    """An AnnotationShape is a shape that represents an annotation."""

    text: Text | None = property_(100)
