from bench.language.const import StructType
from bench.language.module import Struct, struct


@struct(StructType.ACCESS_CONTROL)
class AccessControl(Struct):
    pass


@struct(StructType.ACCESS_CONTROL_RULE)
class AccessControlRule(Struct):
    # principal
    # action
    # resource
    # [condition]
    pass
