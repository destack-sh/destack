from bench.language.core import IsInstantiable, IsModal, IsOwnable, NodeType, PageNode, node_
from bench.pb2 import SceneData

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCENE)
class Scene(IsOwnable, IsInstantiable, IsModal, PageNode[SceneData]):
    pass
