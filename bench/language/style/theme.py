from bench.language.core.node import NodeType, PageNode, node_


@node_(NodeType.THEME)
class Theme(PageNode["ThemeData"]):
    pass
