from typing import Mapping

from bench.language.action import Agency
from bench.language.block import Block
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.flow import StepType
from bench.language.node import Node
from bench.language.text import Text
from bench.language.validation import constraint

BUILTINS = Block.new(BlockType.PAGE, "Builtins")

#
# Implementations (stubs)
# (adding schemas and the like for every :RichBuiltin)
#

IMPLEMENTATIONS = Block.new(BlockType.PAGE, "Implementations")
BUILTINS.blocks.append(IMPLEMENTATIONS)

# step
TRIGGER_STEP = Block.new(BlockType.ACTION, "Trigger", agency=Agency.CODE)
CREATE_STEP = Block.new(
    BlockType.ACTION,
    "Create",
    agency=Agency.CODE,
    fields=(
        Field.input("Input", Node, is_required=True, constraint=constraint(node_is_attached=False)),
        Field.output("Node", Node, is_required=True, constraint=constraint(node_is_attached=True)),
    ),
)
FAIL_STEP = Block.new(
    BlockType.ACTION,
    "Fail",
    agency=Agency.CODE,
    fields=(
        Field.input("title", str),
        Field.input("text", Text),
        Field.input("node", Node),
    ),
)
STUB_BY_STEP_TYPE: Mapping[StepType, Block] = {
    StepType.CREATE: CREATE_STEP,
    StepType.FAIL: FAIL_STEP,
    StepType.TRIGGER: TRIGGER_STEP,
}
IMPLEMENTATIONS.blocks.extend(*STUB_BY_STEP_TYPE.values())

#
# Computer
#

COMPUTER = Block.new(BlockType.PAGE, "Computer")
BUILTINS.blocks.append(COMPUTER)

# nocheckin: assign builtin ids/cks (path+version?)
