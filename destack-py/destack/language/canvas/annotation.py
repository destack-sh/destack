from destack.language.core import NodeType, Text, builtin_node, property_

from .shape import Shape

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ANNOTATION_SHAPE, pretend_frozen=True)
class AnnotationShape(Shape):
    """An AnnotationShape is a shape that represents an annotation."""

    text: Text | None = property_(100)
