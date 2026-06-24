# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

"""Operation executed by one lowered VM instruction."""
Op: typing.TypeAlias = (
    typing.Literal["loadConstCell"]
    | typing.Literal["loadConstAggregate"]
    | typing.Literal["moveCell"]
    | typing.Literal["moveAggregate"]
    | typing.Literal["loadHeapAggregate"]
    | typing.Literal["loadSharedHeapAggregate"]
    | typing.Literal["loadRawAggregate"]
    | typing.Literal["loadStackAggregate"]
    | typing.Literal["loadFrameAggregate"]
    | typing.Literal["loadStaticAggregate"]
    | typing.Literal["storeHeapAggregate"]
    | typing.Literal["storeSharedHeapAggregate"]
    | typing.Literal["storeRawAggregate"]
    | typing.Literal["storeStackAggregate"]
    | typing.Literal["storeFrameAggregate"]
    | typing.Literal["storeStaticAggregate"]
    | typing.Literal["selectCell"]
    | typing.Literal["selectAggregate"]
    | typing.Literal["localAddress"]
    | typing.Literal["staticAddress"]
    | typing.Literal["functionAddress"]
    | typing.Literal["functionBind"]
    | typing.Literal["functionPointer"]
    | typing.Literal["functionEnvironment"]
    | typing.Literal["functionEnvironmentCurrent"]
    | typing.Literal["loadHeapU8"]
    | typing.Literal["loadHeapI8"]
    | typing.Literal["loadHeapU16"]
    | typing.Literal["loadHeapI16"]
    | typing.Literal["loadHeapU32"]
    | typing.Literal["loadHeapI32"]
    | typing.Literal["loadHeap64"]
    | typing.Literal["loadSharedHeapU8"]
    | typing.Literal["loadSharedHeapI8"]
    | typing.Literal["loadSharedHeapU16"]
    | typing.Literal["loadSharedHeapI16"]
    | typing.Literal["loadSharedHeapU32"]
    | typing.Literal["loadSharedHeapI32"]
    | typing.Literal["loadSharedHeap64"]
    | typing.Literal["loadRawU8"]
    | typing.Literal["loadRawI8"]
    | typing.Literal["loadRawU16"]
    | typing.Literal["loadRawI16"]
    | typing.Literal["loadRawU32"]
    | typing.Literal["loadRawI32"]
    | typing.Literal["loadRaw64"]
    | typing.Literal["loadStackU8"]
    | typing.Literal["loadStackI8"]
    | typing.Literal["loadStackU16"]
    | typing.Literal["loadStackI16"]
    | typing.Literal["loadStackU32"]
    | typing.Literal["loadStackI32"]
    | typing.Literal["loadStack64"]
    | typing.Literal["loadFrameU8"]
    | typing.Literal["loadFrameI8"]
    | typing.Literal["loadFrameU16"]
    | typing.Literal["loadFrameI16"]
    | typing.Literal["loadFrameU32"]
    | typing.Literal["loadFrameI32"]
    | typing.Literal["loadFrame64"]
    | typing.Literal["loadFrameValueU8"]
    | typing.Literal["loadFrameValueI8"]
    | typing.Literal["loadFrameValueU16"]
    | typing.Literal["loadFrameValueI16"]
    | typing.Literal["loadFrameValueU32"]
    | typing.Literal["loadFrameValueI32"]
    | typing.Literal["loadFrameValue64"]
    | typing.Literal["loadStaticU8"]
    | typing.Literal["loadStaticI8"]
    | typing.Literal["loadStaticU16"]
    | typing.Literal["loadStaticI16"]
    | typing.Literal["loadStaticU32"]
    | typing.Literal["loadStaticI32"]
    | typing.Literal["loadStatic64"]
    | typing.Literal["storeHeap8"]
    | typing.Literal["storeHeap16"]
    | typing.Literal["storeHeap32"]
    | typing.Literal["storeHeap64"]
    | typing.Literal["storeSharedHeap8"]
    | typing.Literal["storeSharedHeap16"]
    | typing.Literal["storeSharedHeap32"]
    | typing.Literal["storeSharedHeap64"]
    | typing.Literal["storeRaw8"]
    | typing.Literal["storeRaw16"]
    | typing.Literal["storeRaw32"]
    | typing.Literal["storeRaw64"]
    | typing.Literal["storeStack8"]
    | typing.Literal["storeStack16"]
    | typing.Literal["storeStack32"]
    | typing.Literal["storeStack64"]
    | typing.Literal["storeFrame8"]
    | typing.Literal["storeFrame16"]
    | typing.Literal["storeFrame32"]
    | typing.Literal["storeFrame64"]
    | typing.Literal["storeFrameValue8"]
    | typing.Literal["storeFrameValue16"]
    | typing.Literal["storeFrameValue32"]
    | typing.Literal["storeFrameValue64"]
    | typing.Literal["storeStatic8"]
    | typing.Literal["storeStatic16"]
    | typing.Literal["storeStatic32"]
    | typing.Literal["storeStatic64"]
    | typing.Literal["addressFrameValueOffset"]
    | typing.Literal["addressFrameValueElement"]
    | typing.Literal["addressFrameOffset"]
    | typing.Literal["addressHeapOffset"]
    | typing.Literal["addressSharedHeapOffset"]
    | typing.Literal["addressRawOffset"]
    | typing.Literal["addressStackOffset"]
    | typing.Literal["staticAddressOffset"]
    | typing.Literal["addressHeapElement"]
    | typing.Literal["addressSharedHeapElement"]
    | typing.Literal["addressRawElement"]
    | typing.Literal["addressStackElement"]
    | typing.Literal["addressFrameElement"]
    | typing.Literal["staticAddressElement"]
    | typing.Literal["addressHeapSliceElement"]
    | typing.Literal["addressSharedHeapSliceElement"]
    | typing.Literal["addressRawSliceElement"]
    | typing.Literal["addressStackSliceElement"]
    | typing.Literal["addressFrameSliceElement"]
    | typing.Literal["staticAddressSliceElement"]
    | typing.Literal["allocateHeapZeroed"]
    | typing.Literal["allocateHeapUninit"]
    | typing.Literal["allocateHeapSmallNoscanZeroed"]
    | typing.Literal["allocateHeapSmallNoscanUninit"]
    | typing.Literal["allocateHeapSmallScanZeroed"]
    | typing.Literal["allocateHeapSmallScanUninit"]
    | typing.Literal["allocateHeapSmallSharedEdgeZeroed"]
    | typing.Literal["allocateHeapSmallSharedEdgeUninit"]
    | typing.Literal["allocateSharedHeapZeroed"]
    | typing.Literal["allocateSharedHeapUninit"]
    | typing.Literal["allocateSharedHeapSmallZeroed"]
    | typing.Literal["allocateSharedHeapSmallUninit"]
    | typing.Literal["allocateHeapZeroedBranch"]
    | typing.Literal["allocateHeapUninitBranch"]
    | typing.Literal["allocateSharedHeapZeroedBranch"]
    | typing.Literal["allocateSharedHeapUninitBranch"]
    | typing.Literal["allocateSliceZeroed"]
    | typing.Literal["allocateSliceUninit"]
    | typing.Literal["allocateSharedSliceZeroed"]
    | typing.Literal["allocateSharedSliceUninit"]
    | typing.Literal["allocateSliceZeroedBranch"]
    | typing.Literal["allocateSliceUninitBranch"]
    | typing.Literal["allocateSharedSliceZeroedBranch"]
    | typing.Literal["allocateSharedSliceUninitBranch"]
    | typing.Literal["freeHeap"]
    | typing.Literal["freeSharedHeap"]
    | typing.Literal["allocateStackZeroed"]
    | typing.Literal["allocateStackUninit"]
    | typing.Literal["pinHeap"]
    | typing.Literal["pinSharedHeap"]
    | typing.Literal["unpinHeap"]
    | typing.Literal["unpinSharedHeap"]
    | typing.Literal["vectorBinary"]
    | typing.Literal["packedAdd32x4"]
    | typing.Literal["packedSub32x4"]
    | typing.Literal["packedMul32x4"]
    | typing.Literal["packedAnd32x4"]
    | typing.Literal["packedOr32x4"]
    | typing.Literal["packedXor32x4"]
    | typing.Literal["packedShl32x4"]
    | typing.Literal["packedShrI32x4"]
    | typing.Literal["packedShrU32x4"]
    | typing.Literal["packedAdd64x2"]
    | typing.Literal["packedSub64x2"]
    | typing.Literal["packedMul64x2"]
    | typing.Literal["packedAnd64x2"]
    | typing.Literal["packedOr64x2"]
    | typing.Literal["packedXor64x2"]
    | typing.Literal["packedShl64x2"]
    | typing.Literal["packedShrI64x2"]
    | typing.Literal["packedShrU64x2"]
    | typing.Literal["packedAddF32x4"]
    | typing.Literal["packedSubF32x4"]
    | typing.Literal["packedMulF32x4"]
    | typing.Literal["packedDivF32x4"]
    | typing.Literal["packedAddF64x2"]
    | typing.Literal["packedSubF64x2"]
    | typing.Literal["packedMulF64x2"]
    | typing.Literal["packedDivF64x2"]
    | typing.Literal["tensorBinary"]
    | typing.Literal["tensorContiguousBinary"]
    | typing.Literal["andBool"]
    | typing.Literal["orBool"]
    | typing.Literal["xorBool"]
    | typing.Literal["addI32"]
    | typing.Literal["addU32"]
    | typing.Literal["addI64"]
    | typing.Literal["addU64"]
    | typing.Literal["subI32"]
    | typing.Literal["subU32"]
    | typing.Literal["subI64"]
    | typing.Literal["subU64"]
    | typing.Literal["mulI32"]
    | typing.Literal["mulU32"]
    | typing.Literal["mulI64"]
    | typing.Literal["mulU64"]
    | typing.Literal["divI32"]
    | typing.Literal["divU32"]
    | typing.Literal["divI64"]
    | typing.Literal["divU64"]
    | typing.Literal["remI32"]
    | typing.Literal["remU32"]
    | typing.Literal["remI64"]
    | typing.Literal["remU64"]
    | typing.Literal["addCellInt"]
    | typing.Literal["addCellUint"]
    | typing.Literal["subCellInt"]
    | typing.Literal["subCellUint"]
    | typing.Literal["mulCellInt"]
    | typing.Literal["mulCellUint"]
    | typing.Literal["divCellInt"]
    | typing.Literal["divCellUint"]
    | typing.Literal["remCellInt"]
    | typing.Literal["remCellUint"]
    | typing.Literal["and32"]
    | typing.Literal["and64"]
    | typing.Literal["or32"]
    | typing.Literal["or64"]
    | typing.Literal["xor32"]
    | typing.Literal["xor64"]
    | typing.Literal["shl32"]
    | typing.Literal["shl64"]
    | typing.Literal["shrI32"]
    | typing.Literal["shrU32"]
    | typing.Literal["shrI64"]
    | typing.Literal["shrU64"]
    | typing.Literal["addWideInt"]
    | typing.Literal["subWideInt"]
    | typing.Literal["mulWideInt"]
    | typing.Literal["divWideInt"]
    | typing.Literal["divWideUint"]
    | typing.Literal["remWideInt"]
    | typing.Literal["remWideUint"]
    | typing.Literal["andCell"]
    | typing.Literal["orCell"]
    | typing.Literal["xorCell"]
    | typing.Literal["shlCell"]
    | typing.Literal["shrCellInt"]
    | typing.Literal["shrCellUint"]
    | typing.Literal["andWideInt"]
    | typing.Literal["orWideInt"]
    | typing.Literal["xorWideInt"]
    | typing.Literal["shlWideInt"]
    | typing.Literal["shrWideInt"]
    | typing.Literal["shrWideUint"]
    | typing.Literal["addF32"]
    | typing.Literal["addF64"]
    | typing.Literal["subF32"]
    | typing.Literal["subF64"]
    | typing.Literal["mulF32"]
    | typing.Literal["mulF64"]
    | typing.Literal["divF32"]
    | typing.Literal["divF64"]
    | typing.Literal["binaryFloat"]
    | typing.Literal["eq32"]
    | typing.Literal["eq64"]
    | typing.Literal["ne32"]
    | typing.Literal["ne64"]
    | typing.Literal["ltI32"]
    | typing.Literal["ltU32"]
    | typing.Literal["ltI64"]
    | typing.Literal["ltU64"]
    | typing.Literal["leI32"]
    | typing.Literal["leU32"]
    | typing.Literal["leI64"]
    | typing.Literal["leU64"]
    | typing.Literal["gtI32"]
    | typing.Literal["gtU32"]
    | typing.Literal["gtI64"]
    | typing.Literal["gtU64"]
    | typing.Literal["geI32"]
    | typing.Literal["geU32"]
    | typing.Literal["geI64"]
    | typing.Literal["geU64"]
    | typing.Literal["eqCell"]
    | typing.Literal["neCell"]
    | typing.Literal["ltCellInt"]
    | typing.Literal["ltCellUint"]
    | typing.Literal["leCellInt"]
    | typing.Literal["leCellUint"]
    | typing.Literal["gtCellInt"]
    | typing.Literal["gtCellUint"]
    | typing.Literal["geCellInt"]
    | typing.Literal["geCellUint"]
    | typing.Literal["eqWideInt"]
    | typing.Literal["neWideInt"]
    | typing.Literal["ltWideInt"]
    | typing.Literal["ltWideUint"]
    | typing.Literal["leWideInt"]
    | typing.Literal["leWideUint"]
    | typing.Literal["gtWideInt"]
    | typing.Literal["gtWideUint"]
    | typing.Literal["geWideInt"]
    | typing.Literal["geWideUint"]
    | typing.Literal["eqF32"]
    | typing.Literal["eqF64"]
    | typing.Literal["neF32"]
    | typing.Literal["neF64"]
    | typing.Literal["ltF32"]
    | typing.Literal["ltF64"]
    | typing.Literal["leF32"]
    | typing.Literal["leF64"]
    | typing.Literal["gtF32"]
    | typing.Literal["gtF64"]
    | typing.Literal["geF32"]
    | typing.Literal["geF64"]
    | typing.Literal["negI32"]
    | typing.Literal["negI64"]
    | typing.Literal["not32"]
    | typing.Literal["not64"]
    | typing.Literal["negCellInt"]
    | typing.Literal["notCell"]
    | typing.Literal["negWideInt"]
    | typing.Literal["notWideInt"]
    | typing.Literal["negF32"]
    | typing.Literal["negF64"]
    | typing.Literal["unaryFloat"]
    | typing.Literal["notBool"]
    | typing.Literal["vectorUnary"]
    | typing.Literal["tensorUnary"]
    | typing.Literal["packedNegI32x4"]
    | typing.Literal["packedNot32x4"]
    | typing.Literal["packedNegI64x2"]
    | typing.Literal["packedNot64x2"]
    | typing.Literal["packedNegF32x4"]
    | typing.Literal["packedNegF64x2"]
    | typing.Literal["tensorContiguousUnary"]
    | typing.Literal["castBitcast"]
    | typing.Literal["castTruncate"]
    | typing.Literal["castZeroExtend"]
    | typing.Literal["castSignExtend"]
    | typing.Literal["castFloatToSignedInt"]
    | typing.Literal["castFloatToUnsignedInt"]
    | typing.Literal["castFloatToSignedIntSaturating"]
    | typing.Literal["castFloatToUnsignedIntSaturating"]
    | typing.Literal["castSignedIntToFloat"]
    | typing.Literal["castUnsignedIntToFloat"]
    | typing.Literal["castFloatConvert"]
    | typing.Literal["castPointerToInt"]
    | typing.Literal["castIntToPointer"]
    | typing.Literal["castCellToWideInt"]
    | typing.Literal["castWideIntToCell"]
    | typing.Literal["castWideInt"]
    | typing.Literal["castTensorView"]
    | typing.Literal["call"]
    | typing.Literal["callBranch"]
    | typing.Literal["callFunctionPointer"]
    | typing.Literal["callFunction"]
    | typing.Literal["callFunctionPointerBranch"]
    | typing.Literal["callFunctionBranch"]
    | typing.Literal["callVirtualLocal"]
    | typing.Literal["callVirtualShared"]
    | typing.Literal["callVirtualLocalBranch"]
    | typing.Literal["callVirtualSharedBranch"]
    | typing.Literal["callDynamicLocal"]
    | typing.Literal["callDynamicShared"]
    | typing.Literal["callDynamicLocalBranch"]
    | typing.Literal["callDynamicSharedBranch"]
    | typing.Literal["tailCall"]
    | typing.Literal["tailCallSelf"]
    | typing.Literal["tailCallFunctionPointer"]
    | typing.Literal["tailCallFunction"]
    | typing.Literal["tailCallVirtualLocal"]
    | typing.Literal["tailCallVirtualShared"]
    | typing.Literal["tailCallDynamicLocal"]
    | typing.Literal["tailCallDynamicShared"]
    | typing.Literal["jump"]
    | typing.Literal["branchBool"]
    | typing.Literal["branchEq32"]
    | typing.Literal["branchEq64"]
    | typing.Literal["branchNe32"]
    | typing.Literal["branchNe64"]
    | typing.Literal["branchLtI32"]
    | typing.Literal["branchLtU32"]
    | typing.Literal["branchLtI64"]
    | typing.Literal["branchLtU64"]
    | typing.Literal["branchLeI32"]
    | typing.Literal["branchLeU32"]
    | typing.Literal["branchLeI64"]
    | typing.Literal["branchLeU64"]
    | typing.Literal["branchGtI32"]
    | typing.Literal["branchGtU32"]
    | typing.Literal["branchGtI64"]
    | typing.Literal["branchGtU64"]
    | typing.Literal["branchGeI32"]
    | typing.Literal["branchGeU32"]
    | typing.Literal["branchGeI64"]
    | typing.Literal["branchGeU64"]
    | typing.Literal["branchEqCell"]
    | typing.Literal["branchNeCell"]
    | typing.Literal["branchLtCellInt"]
    | typing.Literal["branchLtCellUint"]
    | typing.Literal["branchLeCellInt"]
    | typing.Literal["branchLeCellUint"]
    | typing.Literal["branchGtCellInt"]
    | typing.Literal["branchGtCellUint"]
    | typing.Literal["branchGeCellInt"]
    | typing.Literal["branchGeCellUint"]
    | typing.Literal["branchEqF32"]
    | typing.Literal["branchEqF64"]
    | typing.Literal["branchNeF32"]
    | typing.Literal["branchNeF64"]
    | typing.Literal["branchLtF32"]
    | typing.Literal["branchLtF64"]
    | typing.Literal["branchLeF32"]
    | typing.Literal["branchLeF64"]
    | typing.Literal["branchGtF32"]
    | typing.Literal["branchGtF64"]
    | typing.Literal["branchGeF32"]
    | typing.Literal["branchGeF64"]
    | typing.Literal["switch"]
    | typing.Literal["switchTable"]
    | typing.Literal["check"]
    | typing.Literal["assume"]
    | typing.Literal["returnCell"]
    | typing.Literal["returnAddress"]
    | typing.Literal["returnVoid"]
    | typing.Literal["yieldCell"]
    | typing.Literal["yieldAddress"]
    | typing.Literal["abort"]
    | typing.Literal["panic"]
    | typing.Literal["panicValue"]
    | typing.Literal["unwindResume"]
    | typing.Literal["unreachable"]
    | typing.Literal["barrierWriteHeap"]
    | typing.Literal["barrierWriteSharedHeap"]
    | typing.Literal["atomicLoad"]
    | typing.Literal["atomicStore"]
    | typing.Literal["atomicExchange"]
    | typing.Literal["atomicCompareExchange"]
    | typing.Literal["atomicReadModifyWrite"]
    | typing.Literal["atomicFence"]
    | typing.Literal["intrinsic"]
    | typing.Literal["vectorSplat"]
    | typing.Literal["packedSplat32x4"]
    | typing.Literal["packedSplat64x2"]
    | typing.Literal["vectorExtract"]
    | typing.Literal["vectorInsert"]
    | typing.Literal["vectorShuffle"]
    | typing.Literal["vectorSelect"]
    | typing.Literal["vectorReduce"]
    | typing.Literal["vectorConvert"]
    | typing.Literal["tensorSplat"]
    | typing.Literal["tensorLoad"]
    | typing.Literal["tensorExtract"]
    | typing.Literal["tensorStore"]
    | typing.Literal["tensorFill"]
    | typing.Literal["tensorCopy"]
    | typing.Literal["tensorReshape"]
    | typing.Literal["tensorBroadcast"]
    | typing.Literal["tensorTranspose"]
    | typing.Literal["tensorSlice"]
    | typing.Literal["tensorPad"]
    | typing.Literal["tensorConcat"]
    | typing.Literal["tensorReduce"]
    | typing.Literal["tensorIndexReduce"]
    | typing.Literal["tensorDot"]
    | typing.Literal["tensorConvolution"]
    | typing.Literal["tensorGather"]
    | typing.Literal["tensorScatter"]
    | typing.Literal["tensorSelect"]
    | typing.Literal["tensorConvert"]
    | typing.Literal["tensorCast"]
    | typing.Literal["tensorView"]
)


def encode_op(writer: BinaryWriter, value: Op) -> None:
    """Encode one Op."""
    if value == "loadConstCell":
        writer.write_unsigned(0)
    elif value == "loadConstAggregate":
        writer.write_unsigned(1)
    elif value == "moveCell":
        writer.write_unsigned(2)
    elif value == "moveAggregate":
        writer.write_unsigned(3)
    elif value == "loadHeapAggregate":
        writer.write_unsigned(4)
    elif value == "loadSharedHeapAggregate":
        writer.write_unsigned(5)
    elif value == "loadRawAggregate":
        writer.write_unsigned(6)
    elif value == "loadStackAggregate":
        writer.write_unsigned(7)
    elif value == "loadFrameAggregate":
        writer.write_unsigned(8)
    elif value == "loadStaticAggregate":
        writer.write_unsigned(9)
    elif value == "storeHeapAggregate":
        writer.write_unsigned(10)
    elif value == "storeSharedHeapAggregate":
        writer.write_unsigned(11)
    elif value == "storeRawAggregate":
        writer.write_unsigned(12)
    elif value == "storeStackAggregate":
        writer.write_unsigned(13)
    elif value == "storeFrameAggregate":
        writer.write_unsigned(14)
    elif value == "storeStaticAggregate":
        writer.write_unsigned(15)
    elif value == "selectCell":
        writer.write_unsigned(16)
    elif value == "selectAggregate":
        writer.write_unsigned(17)
    elif value == "localAddress":
        writer.write_unsigned(18)
    elif value == "staticAddress":
        writer.write_unsigned(19)
    elif value == "functionAddress":
        writer.write_unsigned(20)
    elif value == "functionBind":
        writer.write_unsigned(21)
    elif value == "functionPointer":
        writer.write_unsigned(22)
    elif value == "functionEnvironment":
        writer.write_unsigned(23)
    elif value == "functionEnvironmentCurrent":
        writer.write_unsigned(24)
    elif value == "loadHeapU8":
        writer.write_unsigned(25)
    elif value == "loadHeapI8":
        writer.write_unsigned(26)
    elif value == "loadHeapU16":
        writer.write_unsigned(27)
    elif value == "loadHeapI16":
        writer.write_unsigned(28)
    elif value == "loadHeapU32":
        writer.write_unsigned(29)
    elif value == "loadHeapI32":
        writer.write_unsigned(30)
    elif value == "loadHeap64":
        writer.write_unsigned(31)
    elif value == "loadSharedHeapU8":
        writer.write_unsigned(32)
    elif value == "loadSharedHeapI8":
        writer.write_unsigned(33)
    elif value == "loadSharedHeapU16":
        writer.write_unsigned(34)
    elif value == "loadSharedHeapI16":
        writer.write_unsigned(35)
    elif value == "loadSharedHeapU32":
        writer.write_unsigned(36)
    elif value == "loadSharedHeapI32":
        writer.write_unsigned(37)
    elif value == "loadSharedHeap64":
        writer.write_unsigned(38)
    elif value == "loadRawU8":
        writer.write_unsigned(39)
    elif value == "loadRawI8":
        writer.write_unsigned(40)
    elif value == "loadRawU16":
        writer.write_unsigned(41)
    elif value == "loadRawI16":
        writer.write_unsigned(42)
    elif value == "loadRawU32":
        writer.write_unsigned(43)
    elif value == "loadRawI32":
        writer.write_unsigned(44)
    elif value == "loadRaw64":
        writer.write_unsigned(45)
    elif value == "loadStackU8":
        writer.write_unsigned(46)
    elif value == "loadStackI8":
        writer.write_unsigned(47)
    elif value == "loadStackU16":
        writer.write_unsigned(48)
    elif value == "loadStackI16":
        writer.write_unsigned(49)
    elif value == "loadStackU32":
        writer.write_unsigned(50)
    elif value == "loadStackI32":
        writer.write_unsigned(51)
    elif value == "loadStack64":
        writer.write_unsigned(52)
    elif value == "loadFrameU8":
        writer.write_unsigned(53)
    elif value == "loadFrameI8":
        writer.write_unsigned(54)
    elif value == "loadFrameU16":
        writer.write_unsigned(55)
    elif value == "loadFrameI16":
        writer.write_unsigned(56)
    elif value == "loadFrameU32":
        writer.write_unsigned(57)
    elif value == "loadFrameI32":
        writer.write_unsigned(58)
    elif value == "loadFrame64":
        writer.write_unsigned(59)
    elif value == "loadFrameValueU8":
        writer.write_unsigned(60)
    elif value == "loadFrameValueI8":
        writer.write_unsigned(61)
    elif value == "loadFrameValueU16":
        writer.write_unsigned(62)
    elif value == "loadFrameValueI16":
        writer.write_unsigned(63)
    elif value == "loadFrameValueU32":
        writer.write_unsigned(64)
    elif value == "loadFrameValueI32":
        writer.write_unsigned(65)
    elif value == "loadFrameValue64":
        writer.write_unsigned(66)
    elif value == "loadStaticU8":
        writer.write_unsigned(67)
    elif value == "loadStaticI8":
        writer.write_unsigned(68)
    elif value == "loadStaticU16":
        writer.write_unsigned(69)
    elif value == "loadStaticI16":
        writer.write_unsigned(70)
    elif value == "loadStaticU32":
        writer.write_unsigned(71)
    elif value == "loadStaticI32":
        writer.write_unsigned(72)
    elif value == "loadStatic64":
        writer.write_unsigned(73)
    elif value == "storeHeap8":
        writer.write_unsigned(74)
    elif value == "storeHeap16":
        writer.write_unsigned(75)
    elif value == "storeHeap32":
        writer.write_unsigned(76)
    elif value == "storeHeap64":
        writer.write_unsigned(77)
    elif value == "storeSharedHeap8":
        writer.write_unsigned(78)
    elif value == "storeSharedHeap16":
        writer.write_unsigned(79)
    elif value == "storeSharedHeap32":
        writer.write_unsigned(80)
    elif value == "storeSharedHeap64":
        writer.write_unsigned(81)
    elif value == "storeRaw8":
        writer.write_unsigned(82)
    elif value == "storeRaw16":
        writer.write_unsigned(83)
    elif value == "storeRaw32":
        writer.write_unsigned(84)
    elif value == "storeRaw64":
        writer.write_unsigned(85)
    elif value == "storeStack8":
        writer.write_unsigned(86)
    elif value == "storeStack16":
        writer.write_unsigned(87)
    elif value == "storeStack32":
        writer.write_unsigned(88)
    elif value == "storeStack64":
        writer.write_unsigned(89)
    elif value == "storeFrame8":
        writer.write_unsigned(90)
    elif value == "storeFrame16":
        writer.write_unsigned(91)
    elif value == "storeFrame32":
        writer.write_unsigned(92)
    elif value == "storeFrame64":
        writer.write_unsigned(93)
    elif value == "storeFrameValue8":
        writer.write_unsigned(94)
    elif value == "storeFrameValue16":
        writer.write_unsigned(95)
    elif value == "storeFrameValue32":
        writer.write_unsigned(96)
    elif value == "storeFrameValue64":
        writer.write_unsigned(97)
    elif value == "storeStatic8":
        writer.write_unsigned(98)
    elif value == "storeStatic16":
        writer.write_unsigned(99)
    elif value == "storeStatic32":
        writer.write_unsigned(100)
    elif value == "storeStatic64":
        writer.write_unsigned(101)
    elif value == "addressFrameValueOffset":
        writer.write_unsigned(102)
    elif value == "addressFrameValueElement":
        writer.write_unsigned(103)
    elif value == "addressFrameOffset":
        writer.write_unsigned(104)
    elif value == "addressHeapOffset":
        writer.write_unsigned(105)
    elif value == "addressSharedHeapOffset":
        writer.write_unsigned(106)
    elif value == "addressRawOffset":
        writer.write_unsigned(107)
    elif value == "addressStackOffset":
        writer.write_unsigned(108)
    elif value == "staticAddressOffset":
        writer.write_unsigned(109)
    elif value == "addressHeapElement":
        writer.write_unsigned(110)
    elif value == "addressSharedHeapElement":
        writer.write_unsigned(111)
    elif value == "addressRawElement":
        writer.write_unsigned(112)
    elif value == "addressStackElement":
        writer.write_unsigned(113)
    elif value == "addressFrameElement":
        writer.write_unsigned(114)
    elif value == "staticAddressElement":
        writer.write_unsigned(115)
    elif value == "addressHeapSliceElement":
        writer.write_unsigned(116)
    elif value == "addressSharedHeapSliceElement":
        writer.write_unsigned(117)
    elif value == "addressRawSliceElement":
        writer.write_unsigned(118)
    elif value == "addressStackSliceElement":
        writer.write_unsigned(119)
    elif value == "addressFrameSliceElement":
        writer.write_unsigned(120)
    elif value == "staticAddressSliceElement":
        writer.write_unsigned(121)
    elif value == "allocateHeapZeroed":
        writer.write_unsigned(122)
    elif value == "allocateHeapUninit":
        writer.write_unsigned(123)
    elif value == "allocateHeapSmallNoscanZeroed":
        writer.write_unsigned(124)
    elif value == "allocateHeapSmallNoscanUninit":
        writer.write_unsigned(125)
    elif value == "allocateHeapSmallScanZeroed":
        writer.write_unsigned(126)
    elif value == "allocateHeapSmallScanUninit":
        writer.write_unsigned(127)
    elif value == "allocateHeapSmallSharedEdgeZeroed":
        writer.write_unsigned(128)
    elif value == "allocateHeapSmallSharedEdgeUninit":
        writer.write_unsigned(129)
    elif value == "allocateSharedHeapZeroed":
        writer.write_unsigned(130)
    elif value == "allocateSharedHeapUninit":
        writer.write_unsigned(131)
    elif value == "allocateSharedHeapSmallZeroed":
        writer.write_unsigned(132)
    elif value == "allocateSharedHeapSmallUninit":
        writer.write_unsigned(133)
    elif value == "allocateHeapZeroedBranch":
        writer.write_unsigned(134)
    elif value == "allocateHeapUninitBranch":
        writer.write_unsigned(135)
    elif value == "allocateSharedHeapZeroedBranch":
        writer.write_unsigned(136)
    elif value == "allocateSharedHeapUninitBranch":
        writer.write_unsigned(137)
    elif value == "allocateSliceZeroed":
        writer.write_unsigned(138)
    elif value == "allocateSliceUninit":
        writer.write_unsigned(139)
    elif value == "allocateSharedSliceZeroed":
        writer.write_unsigned(140)
    elif value == "allocateSharedSliceUninit":
        writer.write_unsigned(141)
    elif value == "allocateSliceZeroedBranch":
        writer.write_unsigned(142)
    elif value == "allocateSliceUninitBranch":
        writer.write_unsigned(143)
    elif value == "allocateSharedSliceZeroedBranch":
        writer.write_unsigned(144)
    elif value == "allocateSharedSliceUninitBranch":
        writer.write_unsigned(145)
    elif value == "freeHeap":
        writer.write_unsigned(146)
    elif value == "freeSharedHeap":
        writer.write_unsigned(147)
    elif value == "allocateStackZeroed":
        writer.write_unsigned(148)
    elif value == "allocateStackUninit":
        writer.write_unsigned(149)
    elif value == "pinHeap":
        writer.write_unsigned(150)
    elif value == "pinSharedHeap":
        writer.write_unsigned(151)
    elif value == "unpinHeap":
        writer.write_unsigned(152)
    elif value == "unpinSharedHeap":
        writer.write_unsigned(153)
    elif value == "vectorBinary":
        writer.write_unsigned(154)
    elif value == "packedAdd32x4":
        writer.write_unsigned(155)
    elif value == "packedSub32x4":
        writer.write_unsigned(156)
    elif value == "packedMul32x4":
        writer.write_unsigned(157)
    elif value == "packedAnd32x4":
        writer.write_unsigned(158)
    elif value == "packedOr32x4":
        writer.write_unsigned(159)
    elif value == "packedXor32x4":
        writer.write_unsigned(160)
    elif value == "packedShl32x4":
        writer.write_unsigned(161)
    elif value == "packedShrI32x4":
        writer.write_unsigned(162)
    elif value == "packedShrU32x4":
        writer.write_unsigned(163)
    elif value == "packedAdd64x2":
        writer.write_unsigned(164)
    elif value == "packedSub64x2":
        writer.write_unsigned(165)
    elif value == "packedMul64x2":
        writer.write_unsigned(166)
    elif value == "packedAnd64x2":
        writer.write_unsigned(167)
    elif value == "packedOr64x2":
        writer.write_unsigned(168)
    elif value == "packedXor64x2":
        writer.write_unsigned(169)
    elif value == "packedShl64x2":
        writer.write_unsigned(170)
    elif value == "packedShrI64x2":
        writer.write_unsigned(171)
    elif value == "packedShrU64x2":
        writer.write_unsigned(172)
    elif value == "packedAddF32x4":
        writer.write_unsigned(173)
    elif value == "packedSubF32x4":
        writer.write_unsigned(174)
    elif value == "packedMulF32x4":
        writer.write_unsigned(175)
    elif value == "packedDivF32x4":
        writer.write_unsigned(176)
    elif value == "packedAddF64x2":
        writer.write_unsigned(177)
    elif value == "packedSubF64x2":
        writer.write_unsigned(178)
    elif value == "packedMulF64x2":
        writer.write_unsigned(179)
    elif value == "packedDivF64x2":
        writer.write_unsigned(180)
    elif value == "tensorBinary":
        writer.write_unsigned(181)
    elif value == "tensorContiguousBinary":
        writer.write_unsigned(182)
    elif value == "andBool":
        writer.write_unsigned(183)
    elif value == "orBool":
        writer.write_unsigned(184)
    elif value == "xorBool":
        writer.write_unsigned(185)
    elif value == "addI32":
        writer.write_unsigned(186)
    elif value == "addU32":
        writer.write_unsigned(187)
    elif value == "addI64":
        writer.write_unsigned(188)
    elif value == "addU64":
        writer.write_unsigned(189)
    elif value == "subI32":
        writer.write_unsigned(190)
    elif value == "subU32":
        writer.write_unsigned(191)
    elif value == "subI64":
        writer.write_unsigned(192)
    elif value == "subU64":
        writer.write_unsigned(193)
    elif value == "mulI32":
        writer.write_unsigned(194)
    elif value == "mulU32":
        writer.write_unsigned(195)
    elif value == "mulI64":
        writer.write_unsigned(196)
    elif value == "mulU64":
        writer.write_unsigned(197)
    elif value == "divI32":
        writer.write_unsigned(198)
    elif value == "divU32":
        writer.write_unsigned(199)
    elif value == "divI64":
        writer.write_unsigned(200)
    elif value == "divU64":
        writer.write_unsigned(201)
    elif value == "remI32":
        writer.write_unsigned(202)
    elif value == "remU32":
        writer.write_unsigned(203)
    elif value == "remI64":
        writer.write_unsigned(204)
    elif value == "remU64":
        writer.write_unsigned(205)
    elif value == "addCellInt":
        writer.write_unsigned(206)
    elif value == "addCellUint":
        writer.write_unsigned(207)
    elif value == "subCellInt":
        writer.write_unsigned(208)
    elif value == "subCellUint":
        writer.write_unsigned(209)
    elif value == "mulCellInt":
        writer.write_unsigned(210)
    elif value == "mulCellUint":
        writer.write_unsigned(211)
    elif value == "divCellInt":
        writer.write_unsigned(212)
    elif value == "divCellUint":
        writer.write_unsigned(213)
    elif value == "remCellInt":
        writer.write_unsigned(214)
    elif value == "remCellUint":
        writer.write_unsigned(215)
    elif value == "and32":
        writer.write_unsigned(216)
    elif value == "and64":
        writer.write_unsigned(217)
    elif value == "or32":
        writer.write_unsigned(218)
    elif value == "or64":
        writer.write_unsigned(219)
    elif value == "xor32":
        writer.write_unsigned(220)
    elif value == "xor64":
        writer.write_unsigned(221)
    elif value == "shl32":
        writer.write_unsigned(222)
    elif value == "shl64":
        writer.write_unsigned(223)
    elif value == "shrI32":
        writer.write_unsigned(224)
    elif value == "shrU32":
        writer.write_unsigned(225)
    elif value == "shrI64":
        writer.write_unsigned(226)
    elif value == "shrU64":
        writer.write_unsigned(227)
    elif value == "addWideInt":
        writer.write_unsigned(228)
    elif value == "subWideInt":
        writer.write_unsigned(229)
    elif value == "mulWideInt":
        writer.write_unsigned(230)
    elif value == "divWideInt":
        writer.write_unsigned(231)
    elif value == "divWideUint":
        writer.write_unsigned(232)
    elif value == "remWideInt":
        writer.write_unsigned(233)
    elif value == "remWideUint":
        writer.write_unsigned(234)
    elif value == "andCell":
        writer.write_unsigned(235)
    elif value == "orCell":
        writer.write_unsigned(236)
    elif value == "xorCell":
        writer.write_unsigned(237)
    elif value == "shlCell":
        writer.write_unsigned(238)
    elif value == "shrCellInt":
        writer.write_unsigned(239)
    elif value == "shrCellUint":
        writer.write_unsigned(240)
    elif value == "andWideInt":
        writer.write_unsigned(241)
    elif value == "orWideInt":
        writer.write_unsigned(242)
    elif value == "xorWideInt":
        writer.write_unsigned(243)
    elif value == "shlWideInt":
        writer.write_unsigned(244)
    elif value == "shrWideInt":
        writer.write_unsigned(245)
    elif value == "shrWideUint":
        writer.write_unsigned(246)
    elif value == "addF32":
        writer.write_unsigned(247)
    elif value == "addF64":
        writer.write_unsigned(248)
    elif value == "subF32":
        writer.write_unsigned(249)
    elif value == "subF64":
        writer.write_unsigned(250)
    elif value == "mulF32":
        writer.write_unsigned(251)
    elif value == "mulF64":
        writer.write_unsigned(252)
    elif value == "divF32":
        writer.write_unsigned(253)
    elif value == "divF64":
        writer.write_unsigned(254)
    elif value == "binaryFloat":
        writer.write_unsigned(255)
    elif value == "eq32":
        writer.write_unsigned(256)
    elif value == "eq64":
        writer.write_unsigned(257)
    elif value == "ne32":
        writer.write_unsigned(258)
    elif value == "ne64":
        writer.write_unsigned(259)
    elif value == "ltI32":
        writer.write_unsigned(260)
    elif value == "ltU32":
        writer.write_unsigned(261)
    elif value == "ltI64":
        writer.write_unsigned(262)
    elif value == "ltU64":
        writer.write_unsigned(263)
    elif value == "leI32":
        writer.write_unsigned(264)
    elif value == "leU32":
        writer.write_unsigned(265)
    elif value == "leI64":
        writer.write_unsigned(266)
    elif value == "leU64":
        writer.write_unsigned(267)
    elif value == "gtI32":
        writer.write_unsigned(268)
    elif value == "gtU32":
        writer.write_unsigned(269)
    elif value == "gtI64":
        writer.write_unsigned(270)
    elif value == "gtU64":
        writer.write_unsigned(271)
    elif value == "geI32":
        writer.write_unsigned(272)
    elif value == "geU32":
        writer.write_unsigned(273)
    elif value == "geI64":
        writer.write_unsigned(274)
    elif value == "geU64":
        writer.write_unsigned(275)
    elif value == "eqCell":
        writer.write_unsigned(276)
    elif value == "neCell":
        writer.write_unsigned(277)
    elif value == "ltCellInt":
        writer.write_unsigned(278)
    elif value == "ltCellUint":
        writer.write_unsigned(279)
    elif value == "leCellInt":
        writer.write_unsigned(280)
    elif value == "leCellUint":
        writer.write_unsigned(281)
    elif value == "gtCellInt":
        writer.write_unsigned(282)
    elif value == "gtCellUint":
        writer.write_unsigned(283)
    elif value == "geCellInt":
        writer.write_unsigned(284)
    elif value == "geCellUint":
        writer.write_unsigned(285)
    elif value == "eqWideInt":
        writer.write_unsigned(286)
    elif value == "neWideInt":
        writer.write_unsigned(287)
    elif value == "ltWideInt":
        writer.write_unsigned(288)
    elif value == "ltWideUint":
        writer.write_unsigned(289)
    elif value == "leWideInt":
        writer.write_unsigned(290)
    elif value == "leWideUint":
        writer.write_unsigned(291)
    elif value == "gtWideInt":
        writer.write_unsigned(292)
    elif value == "gtWideUint":
        writer.write_unsigned(293)
    elif value == "geWideInt":
        writer.write_unsigned(294)
    elif value == "geWideUint":
        writer.write_unsigned(295)
    elif value == "eqF32":
        writer.write_unsigned(296)
    elif value == "eqF64":
        writer.write_unsigned(297)
    elif value == "neF32":
        writer.write_unsigned(298)
    elif value == "neF64":
        writer.write_unsigned(299)
    elif value == "ltF32":
        writer.write_unsigned(300)
    elif value == "ltF64":
        writer.write_unsigned(301)
    elif value == "leF32":
        writer.write_unsigned(302)
    elif value == "leF64":
        writer.write_unsigned(303)
    elif value == "gtF32":
        writer.write_unsigned(304)
    elif value == "gtF64":
        writer.write_unsigned(305)
    elif value == "geF32":
        writer.write_unsigned(306)
    elif value == "geF64":
        writer.write_unsigned(307)
    elif value == "negI32":
        writer.write_unsigned(308)
    elif value == "negI64":
        writer.write_unsigned(309)
    elif value == "not32":
        writer.write_unsigned(310)
    elif value == "not64":
        writer.write_unsigned(311)
    elif value == "negCellInt":
        writer.write_unsigned(312)
    elif value == "notCell":
        writer.write_unsigned(313)
    elif value == "negWideInt":
        writer.write_unsigned(314)
    elif value == "notWideInt":
        writer.write_unsigned(315)
    elif value == "negF32":
        writer.write_unsigned(316)
    elif value == "negF64":
        writer.write_unsigned(317)
    elif value == "unaryFloat":
        writer.write_unsigned(318)
    elif value == "notBool":
        writer.write_unsigned(319)
    elif value == "vectorUnary":
        writer.write_unsigned(320)
    elif value == "tensorUnary":
        writer.write_unsigned(321)
    elif value == "packedNegI32x4":
        writer.write_unsigned(322)
    elif value == "packedNot32x4":
        writer.write_unsigned(323)
    elif value == "packedNegI64x2":
        writer.write_unsigned(324)
    elif value == "packedNot64x2":
        writer.write_unsigned(325)
    elif value == "packedNegF32x4":
        writer.write_unsigned(326)
    elif value == "packedNegF64x2":
        writer.write_unsigned(327)
    elif value == "tensorContiguousUnary":
        writer.write_unsigned(328)
    elif value == "castBitcast":
        writer.write_unsigned(329)
    elif value == "castTruncate":
        writer.write_unsigned(330)
    elif value == "castZeroExtend":
        writer.write_unsigned(331)
    elif value == "castSignExtend":
        writer.write_unsigned(332)
    elif value == "castFloatToSignedInt":
        writer.write_unsigned(333)
    elif value == "castFloatToUnsignedInt":
        writer.write_unsigned(334)
    elif value == "castFloatToSignedIntSaturating":
        writer.write_unsigned(335)
    elif value == "castFloatToUnsignedIntSaturating":
        writer.write_unsigned(336)
    elif value == "castSignedIntToFloat":
        writer.write_unsigned(337)
    elif value == "castUnsignedIntToFloat":
        writer.write_unsigned(338)
    elif value == "castFloatConvert":
        writer.write_unsigned(339)
    elif value == "castPointerToInt":
        writer.write_unsigned(340)
    elif value == "castIntToPointer":
        writer.write_unsigned(341)
    elif value == "castCellToWideInt":
        writer.write_unsigned(342)
    elif value == "castWideIntToCell":
        writer.write_unsigned(343)
    elif value == "castWideInt":
        writer.write_unsigned(344)
    elif value == "castTensorView":
        writer.write_unsigned(345)
    elif value == "call":
        writer.write_unsigned(346)
    elif value == "callBranch":
        writer.write_unsigned(347)
    elif value == "callFunctionPointer":
        writer.write_unsigned(348)
    elif value == "callFunction":
        writer.write_unsigned(349)
    elif value == "callFunctionPointerBranch":
        writer.write_unsigned(350)
    elif value == "callFunctionBranch":
        writer.write_unsigned(351)
    elif value == "callVirtualLocal":
        writer.write_unsigned(352)
    elif value == "callVirtualShared":
        writer.write_unsigned(353)
    elif value == "callVirtualLocalBranch":
        writer.write_unsigned(354)
    elif value == "callVirtualSharedBranch":
        writer.write_unsigned(355)
    elif value == "callDynamicLocal":
        writer.write_unsigned(356)
    elif value == "callDynamicShared":
        writer.write_unsigned(357)
    elif value == "callDynamicLocalBranch":
        writer.write_unsigned(358)
    elif value == "callDynamicSharedBranch":
        writer.write_unsigned(359)
    elif value == "tailCall":
        writer.write_unsigned(360)
    elif value == "tailCallSelf":
        writer.write_unsigned(361)
    elif value == "tailCallFunctionPointer":
        writer.write_unsigned(362)
    elif value == "tailCallFunction":
        writer.write_unsigned(363)
    elif value == "tailCallVirtualLocal":
        writer.write_unsigned(364)
    elif value == "tailCallVirtualShared":
        writer.write_unsigned(365)
    elif value == "tailCallDynamicLocal":
        writer.write_unsigned(366)
    elif value == "tailCallDynamicShared":
        writer.write_unsigned(367)
    elif value == "jump":
        writer.write_unsigned(368)
    elif value == "branchBool":
        writer.write_unsigned(369)
    elif value == "branchEq32":
        writer.write_unsigned(370)
    elif value == "branchEq64":
        writer.write_unsigned(371)
    elif value == "branchNe32":
        writer.write_unsigned(372)
    elif value == "branchNe64":
        writer.write_unsigned(373)
    elif value == "branchLtI32":
        writer.write_unsigned(374)
    elif value == "branchLtU32":
        writer.write_unsigned(375)
    elif value == "branchLtI64":
        writer.write_unsigned(376)
    elif value == "branchLtU64":
        writer.write_unsigned(377)
    elif value == "branchLeI32":
        writer.write_unsigned(378)
    elif value == "branchLeU32":
        writer.write_unsigned(379)
    elif value == "branchLeI64":
        writer.write_unsigned(380)
    elif value == "branchLeU64":
        writer.write_unsigned(381)
    elif value == "branchGtI32":
        writer.write_unsigned(382)
    elif value == "branchGtU32":
        writer.write_unsigned(383)
    elif value == "branchGtI64":
        writer.write_unsigned(384)
    elif value == "branchGtU64":
        writer.write_unsigned(385)
    elif value == "branchGeI32":
        writer.write_unsigned(386)
    elif value == "branchGeU32":
        writer.write_unsigned(387)
    elif value == "branchGeI64":
        writer.write_unsigned(388)
    elif value == "branchGeU64":
        writer.write_unsigned(389)
    elif value == "branchEqCell":
        writer.write_unsigned(390)
    elif value == "branchNeCell":
        writer.write_unsigned(391)
    elif value == "branchLtCellInt":
        writer.write_unsigned(392)
    elif value == "branchLtCellUint":
        writer.write_unsigned(393)
    elif value == "branchLeCellInt":
        writer.write_unsigned(394)
    elif value == "branchLeCellUint":
        writer.write_unsigned(395)
    elif value == "branchGtCellInt":
        writer.write_unsigned(396)
    elif value == "branchGtCellUint":
        writer.write_unsigned(397)
    elif value == "branchGeCellInt":
        writer.write_unsigned(398)
    elif value == "branchGeCellUint":
        writer.write_unsigned(399)
    elif value == "branchEqF32":
        writer.write_unsigned(400)
    elif value == "branchEqF64":
        writer.write_unsigned(401)
    elif value == "branchNeF32":
        writer.write_unsigned(402)
    elif value == "branchNeF64":
        writer.write_unsigned(403)
    elif value == "branchLtF32":
        writer.write_unsigned(404)
    elif value == "branchLtF64":
        writer.write_unsigned(405)
    elif value == "branchLeF32":
        writer.write_unsigned(406)
    elif value == "branchLeF64":
        writer.write_unsigned(407)
    elif value == "branchGtF32":
        writer.write_unsigned(408)
    elif value == "branchGtF64":
        writer.write_unsigned(409)
    elif value == "branchGeF32":
        writer.write_unsigned(410)
    elif value == "branchGeF64":
        writer.write_unsigned(411)
    elif value == "switch":
        writer.write_unsigned(412)
    elif value == "switchTable":
        writer.write_unsigned(413)
    elif value == "check":
        writer.write_unsigned(414)
    elif value == "assume":
        writer.write_unsigned(415)
    elif value == "returnCell":
        writer.write_unsigned(416)
    elif value == "returnAddress":
        writer.write_unsigned(417)
    elif value == "returnVoid":
        writer.write_unsigned(418)
    elif value == "yieldCell":
        writer.write_unsigned(419)
    elif value == "yieldAddress":
        writer.write_unsigned(420)
    elif value == "abort":
        writer.write_unsigned(421)
    elif value == "panic":
        writer.write_unsigned(422)
    elif value == "panicValue":
        writer.write_unsigned(423)
    elif value == "unwindResume":
        writer.write_unsigned(424)
    elif value == "unreachable":
        writer.write_unsigned(425)
    elif value == "barrierWriteHeap":
        writer.write_unsigned(426)
    elif value == "barrierWriteSharedHeap":
        writer.write_unsigned(427)
    elif value == "atomicLoad":
        writer.write_unsigned(428)
    elif value == "atomicStore":
        writer.write_unsigned(429)
    elif value == "atomicExchange":
        writer.write_unsigned(430)
    elif value == "atomicCompareExchange":
        writer.write_unsigned(431)
    elif value == "atomicReadModifyWrite":
        writer.write_unsigned(432)
    elif value == "atomicFence":
        writer.write_unsigned(433)
    elif value == "intrinsic":
        writer.write_unsigned(434)
    elif value == "vectorSplat":
        writer.write_unsigned(435)
    elif value == "packedSplat32x4":
        writer.write_unsigned(436)
    elif value == "packedSplat64x2":
        writer.write_unsigned(437)
    elif value == "vectorExtract":
        writer.write_unsigned(438)
    elif value == "vectorInsert":
        writer.write_unsigned(439)
    elif value == "vectorShuffle":
        writer.write_unsigned(440)
    elif value == "vectorSelect":
        writer.write_unsigned(441)
    elif value == "vectorReduce":
        writer.write_unsigned(442)
    elif value == "vectorConvert":
        writer.write_unsigned(443)
    elif value == "tensorSplat":
        writer.write_unsigned(444)
    elif value == "tensorLoad":
        writer.write_unsigned(445)
    elif value == "tensorExtract":
        writer.write_unsigned(446)
    elif value == "tensorStore":
        writer.write_unsigned(447)
    elif value == "tensorFill":
        writer.write_unsigned(448)
    elif value == "tensorCopy":
        writer.write_unsigned(449)
    elif value == "tensorReshape":
        writer.write_unsigned(450)
    elif value == "tensorBroadcast":
        writer.write_unsigned(451)
    elif value == "tensorTranspose":
        writer.write_unsigned(452)
    elif value == "tensorSlice":
        writer.write_unsigned(453)
    elif value == "tensorPad":
        writer.write_unsigned(454)
    elif value == "tensorConcat":
        writer.write_unsigned(455)
    elif value == "tensorReduce":
        writer.write_unsigned(456)
    elif value == "tensorIndexReduce":
        writer.write_unsigned(457)
    elif value == "tensorDot":
        writer.write_unsigned(458)
    elif value == "tensorConvolution":
        writer.write_unsigned(459)
    elif value == "tensorGather":
        writer.write_unsigned(460)
    elif value == "tensorScatter":
        writer.write_unsigned(461)
    elif value == "tensorSelect":
        writer.write_unsigned(462)
    elif value == "tensorConvert":
        writer.write_unsigned(463)
    elif value == "tensorCast":
        writer.write_unsigned(464)
    elif value == "tensorView":
        writer.write_unsigned(465)
    else:
        raise SerdeError("unknown enum variant")


def decode_op(reader: BinaryReader) -> Op:
    """Decode one Op."""
    variant = reader.read_number()

    if variant == 0:
        return "loadConstCell"
    elif variant == 1:
        return "loadConstAggregate"
    elif variant == 2:
        return "moveCell"
    elif variant == 3:
        return "moveAggregate"
    elif variant == 4:
        return "loadHeapAggregate"
    elif variant == 5:
        return "loadSharedHeapAggregate"
    elif variant == 6:
        return "loadRawAggregate"
    elif variant == 7:
        return "loadStackAggregate"
    elif variant == 8:
        return "loadFrameAggregate"
    elif variant == 9:
        return "loadStaticAggregate"
    elif variant == 10:
        return "storeHeapAggregate"
    elif variant == 11:
        return "storeSharedHeapAggregate"
    elif variant == 12:
        return "storeRawAggregate"
    elif variant == 13:
        return "storeStackAggregate"
    elif variant == 14:
        return "storeFrameAggregate"
    elif variant == 15:
        return "storeStaticAggregate"
    elif variant == 16:
        return "selectCell"
    elif variant == 17:
        return "selectAggregate"
    elif variant == 18:
        return "localAddress"
    elif variant == 19:
        return "staticAddress"
    elif variant == 20:
        return "functionAddress"
    elif variant == 21:
        return "functionBind"
    elif variant == 22:
        return "functionPointer"
    elif variant == 23:
        return "functionEnvironment"
    elif variant == 24:
        return "functionEnvironmentCurrent"
    elif variant == 25:
        return "loadHeapU8"
    elif variant == 26:
        return "loadHeapI8"
    elif variant == 27:
        return "loadHeapU16"
    elif variant == 28:
        return "loadHeapI16"
    elif variant == 29:
        return "loadHeapU32"
    elif variant == 30:
        return "loadHeapI32"
    elif variant == 31:
        return "loadHeap64"
    elif variant == 32:
        return "loadSharedHeapU8"
    elif variant == 33:
        return "loadSharedHeapI8"
    elif variant == 34:
        return "loadSharedHeapU16"
    elif variant == 35:
        return "loadSharedHeapI16"
    elif variant == 36:
        return "loadSharedHeapU32"
    elif variant == 37:
        return "loadSharedHeapI32"
    elif variant == 38:
        return "loadSharedHeap64"
    elif variant == 39:
        return "loadRawU8"
    elif variant == 40:
        return "loadRawI8"
    elif variant == 41:
        return "loadRawU16"
    elif variant == 42:
        return "loadRawI16"
    elif variant == 43:
        return "loadRawU32"
    elif variant == 44:
        return "loadRawI32"
    elif variant == 45:
        return "loadRaw64"
    elif variant == 46:
        return "loadStackU8"
    elif variant == 47:
        return "loadStackI8"
    elif variant == 48:
        return "loadStackU16"
    elif variant == 49:
        return "loadStackI16"
    elif variant == 50:
        return "loadStackU32"
    elif variant == 51:
        return "loadStackI32"
    elif variant == 52:
        return "loadStack64"
    elif variant == 53:
        return "loadFrameU8"
    elif variant == 54:
        return "loadFrameI8"
    elif variant == 55:
        return "loadFrameU16"
    elif variant == 56:
        return "loadFrameI16"
    elif variant == 57:
        return "loadFrameU32"
    elif variant == 58:
        return "loadFrameI32"
    elif variant == 59:
        return "loadFrame64"
    elif variant == 60:
        return "loadFrameValueU8"
    elif variant == 61:
        return "loadFrameValueI8"
    elif variant == 62:
        return "loadFrameValueU16"
    elif variant == 63:
        return "loadFrameValueI16"
    elif variant == 64:
        return "loadFrameValueU32"
    elif variant == 65:
        return "loadFrameValueI32"
    elif variant == 66:
        return "loadFrameValue64"
    elif variant == 67:
        return "loadStaticU8"
    elif variant == 68:
        return "loadStaticI8"
    elif variant == 69:
        return "loadStaticU16"
    elif variant == 70:
        return "loadStaticI16"
    elif variant == 71:
        return "loadStaticU32"
    elif variant == 72:
        return "loadStaticI32"
    elif variant == 73:
        return "loadStatic64"
    elif variant == 74:
        return "storeHeap8"
    elif variant == 75:
        return "storeHeap16"
    elif variant == 76:
        return "storeHeap32"
    elif variant == 77:
        return "storeHeap64"
    elif variant == 78:
        return "storeSharedHeap8"
    elif variant == 79:
        return "storeSharedHeap16"
    elif variant == 80:
        return "storeSharedHeap32"
    elif variant == 81:
        return "storeSharedHeap64"
    elif variant == 82:
        return "storeRaw8"
    elif variant == 83:
        return "storeRaw16"
    elif variant == 84:
        return "storeRaw32"
    elif variant == 85:
        return "storeRaw64"
    elif variant == 86:
        return "storeStack8"
    elif variant == 87:
        return "storeStack16"
    elif variant == 88:
        return "storeStack32"
    elif variant == 89:
        return "storeStack64"
    elif variant == 90:
        return "storeFrame8"
    elif variant == 91:
        return "storeFrame16"
    elif variant == 92:
        return "storeFrame32"
    elif variant == 93:
        return "storeFrame64"
    elif variant == 94:
        return "storeFrameValue8"
    elif variant == 95:
        return "storeFrameValue16"
    elif variant == 96:
        return "storeFrameValue32"
    elif variant == 97:
        return "storeFrameValue64"
    elif variant == 98:
        return "storeStatic8"
    elif variant == 99:
        return "storeStatic16"
    elif variant == 100:
        return "storeStatic32"
    elif variant == 101:
        return "storeStatic64"
    elif variant == 102:
        return "addressFrameValueOffset"
    elif variant == 103:
        return "addressFrameValueElement"
    elif variant == 104:
        return "addressFrameOffset"
    elif variant == 105:
        return "addressHeapOffset"
    elif variant == 106:
        return "addressSharedHeapOffset"
    elif variant == 107:
        return "addressRawOffset"
    elif variant == 108:
        return "addressStackOffset"
    elif variant == 109:
        return "staticAddressOffset"
    elif variant == 110:
        return "addressHeapElement"
    elif variant == 111:
        return "addressSharedHeapElement"
    elif variant == 112:
        return "addressRawElement"
    elif variant == 113:
        return "addressStackElement"
    elif variant == 114:
        return "addressFrameElement"
    elif variant == 115:
        return "staticAddressElement"
    elif variant == 116:
        return "addressHeapSliceElement"
    elif variant == 117:
        return "addressSharedHeapSliceElement"
    elif variant == 118:
        return "addressRawSliceElement"
    elif variant == 119:
        return "addressStackSliceElement"
    elif variant == 120:
        return "addressFrameSliceElement"
    elif variant == 121:
        return "staticAddressSliceElement"
    elif variant == 122:
        return "allocateHeapZeroed"
    elif variant == 123:
        return "allocateHeapUninit"
    elif variant == 124:
        return "allocateHeapSmallNoscanZeroed"
    elif variant == 125:
        return "allocateHeapSmallNoscanUninit"
    elif variant == 126:
        return "allocateHeapSmallScanZeroed"
    elif variant == 127:
        return "allocateHeapSmallScanUninit"
    elif variant == 128:
        return "allocateHeapSmallSharedEdgeZeroed"
    elif variant == 129:
        return "allocateHeapSmallSharedEdgeUninit"
    elif variant == 130:
        return "allocateSharedHeapZeroed"
    elif variant == 131:
        return "allocateSharedHeapUninit"
    elif variant == 132:
        return "allocateSharedHeapSmallZeroed"
    elif variant == 133:
        return "allocateSharedHeapSmallUninit"
    elif variant == 134:
        return "allocateHeapZeroedBranch"
    elif variant == 135:
        return "allocateHeapUninitBranch"
    elif variant == 136:
        return "allocateSharedHeapZeroedBranch"
    elif variant == 137:
        return "allocateSharedHeapUninitBranch"
    elif variant == 138:
        return "allocateSliceZeroed"
    elif variant == 139:
        return "allocateSliceUninit"
    elif variant == 140:
        return "allocateSharedSliceZeroed"
    elif variant == 141:
        return "allocateSharedSliceUninit"
    elif variant == 142:
        return "allocateSliceZeroedBranch"
    elif variant == 143:
        return "allocateSliceUninitBranch"
    elif variant == 144:
        return "allocateSharedSliceZeroedBranch"
    elif variant == 145:
        return "allocateSharedSliceUninitBranch"
    elif variant == 146:
        return "freeHeap"
    elif variant == 147:
        return "freeSharedHeap"
    elif variant == 148:
        return "allocateStackZeroed"
    elif variant == 149:
        return "allocateStackUninit"
    elif variant == 150:
        return "pinHeap"
    elif variant == 151:
        return "pinSharedHeap"
    elif variant == 152:
        return "unpinHeap"
    elif variant == 153:
        return "unpinSharedHeap"
    elif variant == 154:
        return "vectorBinary"
    elif variant == 155:
        return "packedAdd32x4"
    elif variant == 156:
        return "packedSub32x4"
    elif variant == 157:
        return "packedMul32x4"
    elif variant == 158:
        return "packedAnd32x4"
    elif variant == 159:
        return "packedOr32x4"
    elif variant == 160:
        return "packedXor32x4"
    elif variant == 161:
        return "packedShl32x4"
    elif variant == 162:
        return "packedShrI32x4"
    elif variant == 163:
        return "packedShrU32x4"
    elif variant == 164:
        return "packedAdd64x2"
    elif variant == 165:
        return "packedSub64x2"
    elif variant == 166:
        return "packedMul64x2"
    elif variant == 167:
        return "packedAnd64x2"
    elif variant == 168:
        return "packedOr64x2"
    elif variant == 169:
        return "packedXor64x2"
    elif variant == 170:
        return "packedShl64x2"
    elif variant == 171:
        return "packedShrI64x2"
    elif variant == 172:
        return "packedShrU64x2"
    elif variant == 173:
        return "packedAddF32x4"
    elif variant == 174:
        return "packedSubF32x4"
    elif variant == 175:
        return "packedMulF32x4"
    elif variant == 176:
        return "packedDivF32x4"
    elif variant == 177:
        return "packedAddF64x2"
    elif variant == 178:
        return "packedSubF64x2"
    elif variant == 179:
        return "packedMulF64x2"
    elif variant == 180:
        return "packedDivF64x2"
    elif variant == 181:
        return "tensorBinary"
    elif variant == 182:
        return "tensorContiguousBinary"
    elif variant == 183:
        return "andBool"
    elif variant == 184:
        return "orBool"
    elif variant == 185:
        return "xorBool"
    elif variant == 186:
        return "addI32"
    elif variant == 187:
        return "addU32"
    elif variant == 188:
        return "addI64"
    elif variant == 189:
        return "addU64"
    elif variant == 190:
        return "subI32"
    elif variant == 191:
        return "subU32"
    elif variant == 192:
        return "subI64"
    elif variant == 193:
        return "subU64"
    elif variant == 194:
        return "mulI32"
    elif variant == 195:
        return "mulU32"
    elif variant == 196:
        return "mulI64"
    elif variant == 197:
        return "mulU64"
    elif variant == 198:
        return "divI32"
    elif variant == 199:
        return "divU32"
    elif variant == 200:
        return "divI64"
    elif variant == 201:
        return "divU64"
    elif variant == 202:
        return "remI32"
    elif variant == 203:
        return "remU32"
    elif variant == 204:
        return "remI64"
    elif variant == 205:
        return "remU64"
    elif variant == 206:
        return "addCellInt"
    elif variant == 207:
        return "addCellUint"
    elif variant == 208:
        return "subCellInt"
    elif variant == 209:
        return "subCellUint"
    elif variant == 210:
        return "mulCellInt"
    elif variant == 211:
        return "mulCellUint"
    elif variant == 212:
        return "divCellInt"
    elif variant == 213:
        return "divCellUint"
    elif variant == 214:
        return "remCellInt"
    elif variant == 215:
        return "remCellUint"
    elif variant == 216:
        return "and32"
    elif variant == 217:
        return "and64"
    elif variant == 218:
        return "or32"
    elif variant == 219:
        return "or64"
    elif variant == 220:
        return "xor32"
    elif variant == 221:
        return "xor64"
    elif variant == 222:
        return "shl32"
    elif variant == 223:
        return "shl64"
    elif variant == 224:
        return "shrI32"
    elif variant == 225:
        return "shrU32"
    elif variant == 226:
        return "shrI64"
    elif variant == 227:
        return "shrU64"
    elif variant == 228:
        return "addWideInt"
    elif variant == 229:
        return "subWideInt"
    elif variant == 230:
        return "mulWideInt"
    elif variant == 231:
        return "divWideInt"
    elif variant == 232:
        return "divWideUint"
    elif variant == 233:
        return "remWideInt"
    elif variant == 234:
        return "remWideUint"
    elif variant == 235:
        return "andCell"
    elif variant == 236:
        return "orCell"
    elif variant == 237:
        return "xorCell"
    elif variant == 238:
        return "shlCell"
    elif variant == 239:
        return "shrCellInt"
    elif variant == 240:
        return "shrCellUint"
    elif variant == 241:
        return "andWideInt"
    elif variant == 242:
        return "orWideInt"
    elif variant == 243:
        return "xorWideInt"
    elif variant == 244:
        return "shlWideInt"
    elif variant == 245:
        return "shrWideInt"
    elif variant == 246:
        return "shrWideUint"
    elif variant == 247:
        return "addF32"
    elif variant == 248:
        return "addF64"
    elif variant == 249:
        return "subF32"
    elif variant == 250:
        return "subF64"
    elif variant == 251:
        return "mulF32"
    elif variant == 252:
        return "mulF64"
    elif variant == 253:
        return "divF32"
    elif variant == 254:
        return "divF64"
    elif variant == 255:
        return "binaryFloat"
    elif variant == 256:
        return "eq32"
    elif variant == 257:
        return "eq64"
    elif variant == 258:
        return "ne32"
    elif variant == 259:
        return "ne64"
    elif variant == 260:
        return "ltI32"
    elif variant == 261:
        return "ltU32"
    elif variant == 262:
        return "ltI64"
    elif variant == 263:
        return "ltU64"
    elif variant == 264:
        return "leI32"
    elif variant == 265:
        return "leU32"
    elif variant == 266:
        return "leI64"
    elif variant == 267:
        return "leU64"
    elif variant == 268:
        return "gtI32"
    elif variant == 269:
        return "gtU32"
    elif variant == 270:
        return "gtI64"
    elif variant == 271:
        return "gtU64"
    elif variant == 272:
        return "geI32"
    elif variant == 273:
        return "geU32"
    elif variant == 274:
        return "geI64"
    elif variant == 275:
        return "geU64"
    elif variant == 276:
        return "eqCell"
    elif variant == 277:
        return "neCell"
    elif variant == 278:
        return "ltCellInt"
    elif variant == 279:
        return "ltCellUint"
    elif variant == 280:
        return "leCellInt"
    elif variant == 281:
        return "leCellUint"
    elif variant == 282:
        return "gtCellInt"
    elif variant == 283:
        return "gtCellUint"
    elif variant == 284:
        return "geCellInt"
    elif variant == 285:
        return "geCellUint"
    elif variant == 286:
        return "eqWideInt"
    elif variant == 287:
        return "neWideInt"
    elif variant == 288:
        return "ltWideInt"
    elif variant == 289:
        return "ltWideUint"
    elif variant == 290:
        return "leWideInt"
    elif variant == 291:
        return "leWideUint"
    elif variant == 292:
        return "gtWideInt"
    elif variant == 293:
        return "gtWideUint"
    elif variant == 294:
        return "geWideInt"
    elif variant == 295:
        return "geWideUint"
    elif variant == 296:
        return "eqF32"
    elif variant == 297:
        return "eqF64"
    elif variant == 298:
        return "neF32"
    elif variant == 299:
        return "neF64"
    elif variant == 300:
        return "ltF32"
    elif variant == 301:
        return "ltF64"
    elif variant == 302:
        return "leF32"
    elif variant == 303:
        return "leF64"
    elif variant == 304:
        return "gtF32"
    elif variant == 305:
        return "gtF64"
    elif variant == 306:
        return "geF32"
    elif variant == 307:
        return "geF64"
    elif variant == 308:
        return "negI32"
    elif variant == 309:
        return "negI64"
    elif variant == 310:
        return "not32"
    elif variant == 311:
        return "not64"
    elif variant == 312:
        return "negCellInt"
    elif variant == 313:
        return "notCell"
    elif variant == 314:
        return "negWideInt"
    elif variant == 315:
        return "notWideInt"
    elif variant == 316:
        return "negF32"
    elif variant == 317:
        return "negF64"
    elif variant == 318:
        return "unaryFloat"
    elif variant == 319:
        return "notBool"
    elif variant == 320:
        return "vectorUnary"
    elif variant == 321:
        return "tensorUnary"
    elif variant == 322:
        return "packedNegI32x4"
    elif variant == 323:
        return "packedNot32x4"
    elif variant == 324:
        return "packedNegI64x2"
    elif variant == 325:
        return "packedNot64x2"
    elif variant == 326:
        return "packedNegF32x4"
    elif variant == 327:
        return "packedNegF64x2"
    elif variant == 328:
        return "tensorContiguousUnary"
    elif variant == 329:
        return "castBitcast"
    elif variant == 330:
        return "castTruncate"
    elif variant == 331:
        return "castZeroExtend"
    elif variant == 332:
        return "castSignExtend"
    elif variant == 333:
        return "castFloatToSignedInt"
    elif variant == 334:
        return "castFloatToUnsignedInt"
    elif variant == 335:
        return "castFloatToSignedIntSaturating"
    elif variant == 336:
        return "castFloatToUnsignedIntSaturating"
    elif variant == 337:
        return "castSignedIntToFloat"
    elif variant == 338:
        return "castUnsignedIntToFloat"
    elif variant == 339:
        return "castFloatConvert"
    elif variant == 340:
        return "castPointerToInt"
    elif variant == 341:
        return "castIntToPointer"
    elif variant == 342:
        return "castCellToWideInt"
    elif variant == 343:
        return "castWideIntToCell"
    elif variant == 344:
        return "castWideInt"
    elif variant == 345:
        return "castTensorView"
    elif variant == 346:
        return "call"
    elif variant == 347:
        return "callBranch"
    elif variant == 348:
        return "callFunctionPointer"
    elif variant == 349:
        return "callFunction"
    elif variant == 350:
        return "callFunctionPointerBranch"
    elif variant == 351:
        return "callFunctionBranch"
    elif variant == 352:
        return "callVirtualLocal"
    elif variant == 353:
        return "callVirtualShared"
    elif variant == 354:
        return "callVirtualLocalBranch"
    elif variant == 355:
        return "callVirtualSharedBranch"
    elif variant == 356:
        return "callDynamicLocal"
    elif variant == 357:
        return "callDynamicShared"
    elif variant == 358:
        return "callDynamicLocalBranch"
    elif variant == 359:
        return "callDynamicSharedBranch"
    elif variant == 360:
        return "tailCall"
    elif variant == 361:
        return "tailCallSelf"
    elif variant == 362:
        return "tailCallFunctionPointer"
    elif variant == 363:
        return "tailCallFunction"
    elif variant == 364:
        return "tailCallVirtualLocal"
    elif variant == 365:
        return "tailCallVirtualShared"
    elif variant == 366:
        return "tailCallDynamicLocal"
    elif variant == 367:
        return "tailCallDynamicShared"
    elif variant == 368:
        return "jump"
    elif variant == 369:
        return "branchBool"
    elif variant == 370:
        return "branchEq32"
    elif variant == 371:
        return "branchEq64"
    elif variant == 372:
        return "branchNe32"
    elif variant == 373:
        return "branchNe64"
    elif variant == 374:
        return "branchLtI32"
    elif variant == 375:
        return "branchLtU32"
    elif variant == 376:
        return "branchLtI64"
    elif variant == 377:
        return "branchLtU64"
    elif variant == 378:
        return "branchLeI32"
    elif variant == 379:
        return "branchLeU32"
    elif variant == 380:
        return "branchLeI64"
    elif variant == 381:
        return "branchLeU64"
    elif variant == 382:
        return "branchGtI32"
    elif variant == 383:
        return "branchGtU32"
    elif variant == 384:
        return "branchGtI64"
    elif variant == 385:
        return "branchGtU64"
    elif variant == 386:
        return "branchGeI32"
    elif variant == 387:
        return "branchGeU32"
    elif variant == 388:
        return "branchGeI64"
    elif variant == 389:
        return "branchGeU64"
    elif variant == 390:
        return "branchEqCell"
    elif variant == 391:
        return "branchNeCell"
    elif variant == 392:
        return "branchLtCellInt"
    elif variant == 393:
        return "branchLtCellUint"
    elif variant == 394:
        return "branchLeCellInt"
    elif variant == 395:
        return "branchLeCellUint"
    elif variant == 396:
        return "branchGtCellInt"
    elif variant == 397:
        return "branchGtCellUint"
    elif variant == 398:
        return "branchGeCellInt"
    elif variant == 399:
        return "branchGeCellUint"
    elif variant == 400:
        return "branchEqF32"
    elif variant == 401:
        return "branchEqF64"
    elif variant == 402:
        return "branchNeF32"
    elif variant == 403:
        return "branchNeF64"
    elif variant == 404:
        return "branchLtF32"
    elif variant == 405:
        return "branchLtF64"
    elif variant == 406:
        return "branchLeF32"
    elif variant == 407:
        return "branchLeF64"
    elif variant == 408:
        return "branchGtF32"
    elif variant == 409:
        return "branchGtF64"
    elif variant == 410:
        return "branchGeF32"
    elif variant == 411:
        return "branchGeF64"
    elif variant == 412:
        return "switch"
    elif variant == 413:
        return "switchTable"
    elif variant == 414:
        return "check"
    elif variant == 415:
        return "assume"
    elif variant == 416:
        return "returnCell"
    elif variant == 417:
        return "returnAddress"
    elif variant == 418:
        return "returnVoid"
    elif variant == 419:
        return "yieldCell"
    elif variant == 420:
        return "yieldAddress"
    elif variant == 421:
        return "abort"
    elif variant == 422:
        return "panic"
    elif variant == 423:
        return "panicValue"
    elif variant == 424:
        return "unwindResume"
    elif variant == 425:
        return "unreachable"
    elif variant == 426:
        return "barrierWriteHeap"
    elif variant == 427:
        return "barrierWriteSharedHeap"
    elif variant == 428:
        return "atomicLoad"
    elif variant == 429:
        return "atomicStore"
    elif variant == 430:
        return "atomicExchange"
    elif variant == 431:
        return "atomicCompareExchange"
    elif variant == 432:
        return "atomicReadModifyWrite"
    elif variant == 433:
        return "atomicFence"
    elif variant == 434:
        return "intrinsic"
    elif variant == 435:
        return "vectorSplat"
    elif variant == 436:
        return "packedSplat32x4"
    elif variant == 437:
        return "packedSplat64x2"
    elif variant == 438:
        return "vectorExtract"
    elif variant == 439:
        return "vectorInsert"
    elif variant == 440:
        return "vectorShuffle"
    elif variant == 441:
        return "vectorSelect"
    elif variant == 442:
        return "vectorReduce"
    elif variant == 443:
        return "vectorConvert"
    elif variant == 444:
        return "tensorSplat"
    elif variant == 445:
        return "tensorLoad"
    elif variant == 446:
        return "tensorExtract"
    elif variant == 447:
        return "tensorStore"
    elif variant == 448:
        return "tensorFill"
    elif variant == 449:
        return "tensorCopy"
    elif variant == 450:
        return "tensorReshape"
    elif variant == 451:
        return "tensorBroadcast"
    elif variant == 452:
        return "tensorTranspose"
    elif variant == 453:
        return "tensorSlice"
    elif variant == 454:
        return "tensorPad"
    elif variant == 455:
        return "tensorConcat"
    elif variant == 456:
        return "tensorReduce"
    elif variant == 457:
        return "tensorIndexReduce"
    elif variant == 458:
        return "tensorDot"
    elif variant == 459:
        return "tensorConvolution"
    elif variant == 460:
        return "tensorGather"
    elif variant == 461:
        return "tensorScatter"
    elif variant == 462:
        return "tensorSelect"
    elif variant == 463:
        return "tensorConvert"
    elif variant == 464:
        return "tensorCast"
    elif variant == 465:
        return "tensorView"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_op(value: Op) -> Json:
    """Return one JSON value for one Op."""
    return value


def from_json_op(value: Json) -> Op:
    """Return one Op from one JSON value."""
    variant = json_string(value)

    if variant == "loadConstCell":
        return "loadConstCell"
    elif variant == "loadConstAggregate":
        return "loadConstAggregate"
    elif variant == "moveCell":
        return "moveCell"
    elif variant == "moveAggregate":
        return "moveAggregate"
    elif variant == "loadHeapAggregate":
        return "loadHeapAggregate"
    elif variant == "loadSharedHeapAggregate":
        return "loadSharedHeapAggregate"
    elif variant == "loadRawAggregate":
        return "loadRawAggregate"
    elif variant == "loadStackAggregate":
        return "loadStackAggregate"
    elif variant == "loadFrameAggregate":
        return "loadFrameAggregate"
    elif variant == "loadStaticAggregate":
        return "loadStaticAggregate"
    elif variant == "storeHeapAggregate":
        return "storeHeapAggregate"
    elif variant == "storeSharedHeapAggregate":
        return "storeSharedHeapAggregate"
    elif variant == "storeRawAggregate":
        return "storeRawAggregate"
    elif variant == "storeStackAggregate":
        return "storeStackAggregate"
    elif variant == "storeFrameAggregate":
        return "storeFrameAggregate"
    elif variant == "storeStaticAggregate":
        return "storeStaticAggregate"
    elif variant == "selectCell":
        return "selectCell"
    elif variant == "selectAggregate":
        return "selectAggregate"
    elif variant == "localAddress":
        return "localAddress"
    elif variant == "staticAddress":
        return "staticAddress"
    elif variant == "functionAddress":
        return "functionAddress"
    elif variant == "functionBind":
        return "functionBind"
    elif variant == "functionPointer":
        return "functionPointer"
    elif variant == "functionEnvironment":
        return "functionEnvironment"
    elif variant == "functionEnvironmentCurrent":
        return "functionEnvironmentCurrent"
    elif variant == "loadHeapU8":
        return "loadHeapU8"
    elif variant == "loadHeapI8":
        return "loadHeapI8"
    elif variant == "loadHeapU16":
        return "loadHeapU16"
    elif variant == "loadHeapI16":
        return "loadHeapI16"
    elif variant == "loadHeapU32":
        return "loadHeapU32"
    elif variant == "loadHeapI32":
        return "loadHeapI32"
    elif variant == "loadHeap64":
        return "loadHeap64"
    elif variant == "loadSharedHeapU8":
        return "loadSharedHeapU8"
    elif variant == "loadSharedHeapI8":
        return "loadSharedHeapI8"
    elif variant == "loadSharedHeapU16":
        return "loadSharedHeapU16"
    elif variant == "loadSharedHeapI16":
        return "loadSharedHeapI16"
    elif variant == "loadSharedHeapU32":
        return "loadSharedHeapU32"
    elif variant == "loadSharedHeapI32":
        return "loadSharedHeapI32"
    elif variant == "loadSharedHeap64":
        return "loadSharedHeap64"
    elif variant == "loadRawU8":
        return "loadRawU8"
    elif variant == "loadRawI8":
        return "loadRawI8"
    elif variant == "loadRawU16":
        return "loadRawU16"
    elif variant == "loadRawI16":
        return "loadRawI16"
    elif variant == "loadRawU32":
        return "loadRawU32"
    elif variant == "loadRawI32":
        return "loadRawI32"
    elif variant == "loadRaw64":
        return "loadRaw64"
    elif variant == "loadStackU8":
        return "loadStackU8"
    elif variant == "loadStackI8":
        return "loadStackI8"
    elif variant == "loadStackU16":
        return "loadStackU16"
    elif variant == "loadStackI16":
        return "loadStackI16"
    elif variant == "loadStackU32":
        return "loadStackU32"
    elif variant == "loadStackI32":
        return "loadStackI32"
    elif variant == "loadStack64":
        return "loadStack64"
    elif variant == "loadFrameU8":
        return "loadFrameU8"
    elif variant == "loadFrameI8":
        return "loadFrameI8"
    elif variant == "loadFrameU16":
        return "loadFrameU16"
    elif variant == "loadFrameI16":
        return "loadFrameI16"
    elif variant == "loadFrameU32":
        return "loadFrameU32"
    elif variant == "loadFrameI32":
        return "loadFrameI32"
    elif variant == "loadFrame64":
        return "loadFrame64"
    elif variant == "loadFrameValueU8":
        return "loadFrameValueU8"
    elif variant == "loadFrameValueI8":
        return "loadFrameValueI8"
    elif variant == "loadFrameValueU16":
        return "loadFrameValueU16"
    elif variant == "loadFrameValueI16":
        return "loadFrameValueI16"
    elif variant == "loadFrameValueU32":
        return "loadFrameValueU32"
    elif variant == "loadFrameValueI32":
        return "loadFrameValueI32"
    elif variant == "loadFrameValue64":
        return "loadFrameValue64"
    elif variant == "loadStaticU8":
        return "loadStaticU8"
    elif variant == "loadStaticI8":
        return "loadStaticI8"
    elif variant == "loadStaticU16":
        return "loadStaticU16"
    elif variant == "loadStaticI16":
        return "loadStaticI16"
    elif variant == "loadStaticU32":
        return "loadStaticU32"
    elif variant == "loadStaticI32":
        return "loadStaticI32"
    elif variant == "loadStatic64":
        return "loadStatic64"
    elif variant == "storeHeap8":
        return "storeHeap8"
    elif variant == "storeHeap16":
        return "storeHeap16"
    elif variant == "storeHeap32":
        return "storeHeap32"
    elif variant == "storeHeap64":
        return "storeHeap64"
    elif variant == "storeSharedHeap8":
        return "storeSharedHeap8"
    elif variant == "storeSharedHeap16":
        return "storeSharedHeap16"
    elif variant == "storeSharedHeap32":
        return "storeSharedHeap32"
    elif variant == "storeSharedHeap64":
        return "storeSharedHeap64"
    elif variant == "storeRaw8":
        return "storeRaw8"
    elif variant == "storeRaw16":
        return "storeRaw16"
    elif variant == "storeRaw32":
        return "storeRaw32"
    elif variant == "storeRaw64":
        return "storeRaw64"
    elif variant == "storeStack8":
        return "storeStack8"
    elif variant == "storeStack16":
        return "storeStack16"
    elif variant == "storeStack32":
        return "storeStack32"
    elif variant == "storeStack64":
        return "storeStack64"
    elif variant == "storeFrame8":
        return "storeFrame8"
    elif variant == "storeFrame16":
        return "storeFrame16"
    elif variant == "storeFrame32":
        return "storeFrame32"
    elif variant == "storeFrame64":
        return "storeFrame64"
    elif variant == "storeFrameValue8":
        return "storeFrameValue8"
    elif variant == "storeFrameValue16":
        return "storeFrameValue16"
    elif variant == "storeFrameValue32":
        return "storeFrameValue32"
    elif variant == "storeFrameValue64":
        return "storeFrameValue64"
    elif variant == "storeStatic8":
        return "storeStatic8"
    elif variant == "storeStatic16":
        return "storeStatic16"
    elif variant == "storeStatic32":
        return "storeStatic32"
    elif variant == "storeStatic64":
        return "storeStatic64"
    elif variant == "addressFrameValueOffset":
        return "addressFrameValueOffset"
    elif variant == "addressFrameValueElement":
        return "addressFrameValueElement"
    elif variant == "addressFrameOffset":
        return "addressFrameOffset"
    elif variant == "addressHeapOffset":
        return "addressHeapOffset"
    elif variant == "addressSharedHeapOffset":
        return "addressSharedHeapOffset"
    elif variant == "addressRawOffset":
        return "addressRawOffset"
    elif variant == "addressStackOffset":
        return "addressStackOffset"
    elif variant == "staticAddressOffset":
        return "staticAddressOffset"
    elif variant == "addressHeapElement":
        return "addressHeapElement"
    elif variant == "addressSharedHeapElement":
        return "addressSharedHeapElement"
    elif variant == "addressRawElement":
        return "addressRawElement"
    elif variant == "addressStackElement":
        return "addressStackElement"
    elif variant == "addressFrameElement":
        return "addressFrameElement"
    elif variant == "staticAddressElement":
        return "staticAddressElement"
    elif variant == "addressHeapSliceElement":
        return "addressHeapSliceElement"
    elif variant == "addressSharedHeapSliceElement":
        return "addressSharedHeapSliceElement"
    elif variant == "addressRawSliceElement":
        return "addressRawSliceElement"
    elif variant == "addressStackSliceElement":
        return "addressStackSliceElement"
    elif variant == "addressFrameSliceElement":
        return "addressFrameSliceElement"
    elif variant == "staticAddressSliceElement":
        return "staticAddressSliceElement"
    elif variant == "allocateHeapZeroed":
        return "allocateHeapZeroed"
    elif variant == "allocateHeapUninit":
        return "allocateHeapUninit"
    elif variant == "allocateHeapSmallNoscanZeroed":
        return "allocateHeapSmallNoscanZeroed"
    elif variant == "allocateHeapSmallNoscanUninit":
        return "allocateHeapSmallNoscanUninit"
    elif variant == "allocateHeapSmallScanZeroed":
        return "allocateHeapSmallScanZeroed"
    elif variant == "allocateHeapSmallScanUninit":
        return "allocateHeapSmallScanUninit"
    elif variant == "allocateHeapSmallSharedEdgeZeroed":
        return "allocateHeapSmallSharedEdgeZeroed"
    elif variant == "allocateHeapSmallSharedEdgeUninit":
        return "allocateHeapSmallSharedEdgeUninit"
    elif variant == "allocateSharedHeapZeroed":
        return "allocateSharedHeapZeroed"
    elif variant == "allocateSharedHeapUninit":
        return "allocateSharedHeapUninit"
    elif variant == "allocateSharedHeapSmallZeroed":
        return "allocateSharedHeapSmallZeroed"
    elif variant == "allocateSharedHeapSmallUninit":
        return "allocateSharedHeapSmallUninit"
    elif variant == "allocateHeapZeroedBranch":
        return "allocateHeapZeroedBranch"
    elif variant == "allocateHeapUninitBranch":
        return "allocateHeapUninitBranch"
    elif variant == "allocateSharedHeapZeroedBranch":
        return "allocateSharedHeapZeroedBranch"
    elif variant == "allocateSharedHeapUninitBranch":
        return "allocateSharedHeapUninitBranch"
    elif variant == "allocateSliceZeroed":
        return "allocateSliceZeroed"
    elif variant == "allocateSliceUninit":
        return "allocateSliceUninit"
    elif variant == "allocateSharedSliceZeroed":
        return "allocateSharedSliceZeroed"
    elif variant == "allocateSharedSliceUninit":
        return "allocateSharedSliceUninit"
    elif variant == "allocateSliceZeroedBranch":
        return "allocateSliceZeroedBranch"
    elif variant == "allocateSliceUninitBranch":
        return "allocateSliceUninitBranch"
    elif variant == "allocateSharedSliceZeroedBranch":
        return "allocateSharedSliceZeroedBranch"
    elif variant == "allocateSharedSliceUninitBranch":
        return "allocateSharedSliceUninitBranch"
    elif variant == "freeHeap":
        return "freeHeap"
    elif variant == "freeSharedHeap":
        return "freeSharedHeap"
    elif variant == "allocateStackZeroed":
        return "allocateStackZeroed"
    elif variant == "allocateStackUninit":
        return "allocateStackUninit"
    elif variant == "pinHeap":
        return "pinHeap"
    elif variant == "pinSharedHeap":
        return "pinSharedHeap"
    elif variant == "unpinHeap":
        return "unpinHeap"
    elif variant == "unpinSharedHeap":
        return "unpinSharedHeap"
    elif variant == "vectorBinary":
        return "vectorBinary"
    elif variant == "packedAdd32x4":
        return "packedAdd32x4"
    elif variant == "packedSub32x4":
        return "packedSub32x4"
    elif variant == "packedMul32x4":
        return "packedMul32x4"
    elif variant == "packedAnd32x4":
        return "packedAnd32x4"
    elif variant == "packedOr32x4":
        return "packedOr32x4"
    elif variant == "packedXor32x4":
        return "packedXor32x4"
    elif variant == "packedShl32x4":
        return "packedShl32x4"
    elif variant == "packedShrI32x4":
        return "packedShrI32x4"
    elif variant == "packedShrU32x4":
        return "packedShrU32x4"
    elif variant == "packedAdd64x2":
        return "packedAdd64x2"
    elif variant == "packedSub64x2":
        return "packedSub64x2"
    elif variant == "packedMul64x2":
        return "packedMul64x2"
    elif variant == "packedAnd64x2":
        return "packedAnd64x2"
    elif variant == "packedOr64x2":
        return "packedOr64x2"
    elif variant == "packedXor64x2":
        return "packedXor64x2"
    elif variant == "packedShl64x2":
        return "packedShl64x2"
    elif variant == "packedShrI64x2":
        return "packedShrI64x2"
    elif variant == "packedShrU64x2":
        return "packedShrU64x2"
    elif variant == "packedAddF32x4":
        return "packedAddF32x4"
    elif variant == "packedSubF32x4":
        return "packedSubF32x4"
    elif variant == "packedMulF32x4":
        return "packedMulF32x4"
    elif variant == "packedDivF32x4":
        return "packedDivF32x4"
    elif variant == "packedAddF64x2":
        return "packedAddF64x2"
    elif variant == "packedSubF64x2":
        return "packedSubF64x2"
    elif variant == "packedMulF64x2":
        return "packedMulF64x2"
    elif variant == "packedDivF64x2":
        return "packedDivF64x2"
    elif variant == "tensorBinary":
        return "tensorBinary"
    elif variant == "tensorContiguousBinary":
        return "tensorContiguousBinary"
    elif variant == "andBool":
        return "andBool"
    elif variant == "orBool":
        return "orBool"
    elif variant == "xorBool":
        return "xorBool"
    elif variant == "addI32":
        return "addI32"
    elif variant == "addU32":
        return "addU32"
    elif variant == "addI64":
        return "addI64"
    elif variant == "addU64":
        return "addU64"
    elif variant == "subI32":
        return "subI32"
    elif variant == "subU32":
        return "subU32"
    elif variant == "subI64":
        return "subI64"
    elif variant == "subU64":
        return "subU64"
    elif variant == "mulI32":
        return "mulI32"
    elif variant == "mulU32":
        return "mulU32"
    elif variant == "mulI64":
        return "mulI64"
    elif variant == "mulU64":
        return "mulU64"
    elif variant == "divI32":
        return "divI32"
    elif variant == "divU32":
        return "divU32"
    elif variant == "divI64":
        return "divI64"
    elif variant == "divU64":
        return "divU64"
    elif variant == "remI32":
        return "remI32"
    elif variant == "remU32":
        return "remU32"
    elif variant == "remI64":
        return "remI64"
    elif variant == "remU64":
        return "remU64"
    elif variant == "addCellInt":
        return "addCellInt"
    elif variant == "addCellUint":
        return "addCellUint"
    elif variant == "subCellInt":
        return "subCellInt"
    elif variant == "subCellUint":
        return "subCellUint"
    elif variant == "mulCellInt":
        return "mulCellInt"
    elif variant == "mulCellUint":
        return "mulCellUint"
    elif variant == "divCellInt":
        return "divCellInt"
    elif variant == "divCellUint":
        return "divCellUint"
    elif variant == "remCellInt":
        return "remCellInt"
    elif variant == "remCellUint":
        return "remCellUint"
    elif variant == "and32":
        return "and32"
    elif variant == "and64":
        return "and64"
    elif variant == "or32":
        return "or32"
    elif variant == "or64":
        return "or64"
    elif variant == "xor32":
        return "xor32"
    elif variant == "xor64":
        return "xor64"
    elif variant == "shl32":
        return "shl32"
    elif variant == "shl64":
        return "shl64"
    elif variant == "shrI32":
        return "shrI32"
    elif variant == "shrU32":
        return "shrU32"
    elif variant == "shrI64":
        return "shrI64"
    elif variant == "shrU64":
        return "shrU64"
    elif variant == "addWideInt":
        return "addWideInt"
    elif variant == "subWideInt":
        return "subWideInt"
    elif variant == "mulWideInt":
        return "mulWideInt"
    elif variant == "divWideInt":
        return "divWideInt"
    elif variant == "divWideUint":
        return "divWideUint"
    elif variant == "remWideInt":
        return "remWideInt"
    elif variant == "remWideUint":
        return "remWideUint"
    elif variant == "andCell":
        return "andCell"
    elif variant == "orCell":
        return "orCell"
    elif variant == "xorCell":
        return "xorCell"
    elif variant == "shlCell":
        return "shlCell"
    elif variant == "shrCellInt":
        return "shrCellInt"
    elif variant == "shrCellUint":
        return "shrCellUint"
    elif variant == "andWideInt":
        return "andWideInt"
    elif variant == "orWideInt":
        return "orWideInt"
    elif variant == "xorWideInt":
        return "xorWideInt"
    elif variant == "shlWideInt":
        return "shlWideInt"
    elif variant == "shrWideInt":
        return "shrWideInt"
    elif variant == "shrWideUint":
        return "shrWideUint"
    elif variant == "addF32":
        return "addF32"
    elif variant == "addF64":
        return "addF64"
    elif variant == "subF32":
        return "subF32"
    elif variant == "subF64":
        return "subF64"
    elif variant == "mulF32":
        return "mulF32"
    elif variant == "mulF64":
        return "mulF64"
    elif variant == "divF32":
        return "divF32"
    elif variant == "divF64":
        return "divF64"
    elif variant == "binaryFloat":
        return "binaryFloat"
    elif variant == "eq32":
        return "eq32"
    elif variant == "eq64":
        return "eq64"
    elif variant == "ne32":
        return "ne32"
    elif variant == "ne64":
        return "ne64"
    elif variant == "ltI32":
        return "ltI32"
    elif variant == "ltU32":
        return "ltU32"
    elif variant == "ltI64":
        return "ltI64"
    elif variant == "ltU64":
        return "ltU64"
    elif variant == "leI32":
        return "leI32"
    elif variant == "leU32":
        return "leU32"
    elif variant == "leI64":
        return "leI64"
    elif variant == "leU64":
        return "leU64"
    elif variant == "gtI32":
        return "gtI32"
    elif variant == "gtU32":
        return "gtU32"
    elif variant == "gtI64":
        return "gtI64"
    elif variant == "gtU64":
        return "gtU64"
    elif variant == "geI32":
        return "geI32"
    elif variant == "geU32":
        return "geU32"
    elif variant == "geI64":
        return "geI64"
    elif variant == "geU64":
        return "geU64"
    elif variant == "eqCell":
        return "eqCell"
    elif variant == "neCell":
        return "neCell"
    elif variant == "ltCellInt":
        return "ltCellInt"
    elif variant == "ltCellUint":
        return "ltCellUint"
    elif variant == "leCellInt":
        return "leCellInt"
    elif variant == "leCellUint":
        return "leCellUint"
    elif variant == "gtCellInt":
        return "gtCellInt"
    elif variant == "gtCellUint":
        return "gtCellUint"
    elif variant == "geCellInt":
        return "geCellInt"
    elif variant == "geCellUint":
        return "geCellUint"
    elif variant == "eqWideInt":
        return "eqWideInt"
    elif variant == "neWideInt":
        return "neWideInt"
    elif variant == "ltWideInt":
        return "ltWideInt"
    elif variant == "ltWideUint":
        return "ltWideUint"
    elif variant == "leWideInt":
        return "leWideInt"
    elif variant == "leWideUint":
        return "leWideUint"
    elif variant == "gtWideInt":
        return "gtWideInt"
    elif variant == "gtWideUint":
        return "gtWideUint"
    elif variant == "geWideInt":
        return "geWideInt"
    elif variant == "geWideUint":
        return "geWideUint"
    elif variant == "eqF32":
        return "eqF32"
    elif variant == "eqF64":
        return "eqF64"
    elif variant == "neF32":
        return "neF32"
    elif variant == "neF64":
        return "neF64"
    elif variant == "ltF32":
        return "ltF32"
    elif variant == "ltF64":
        return "ltF64"
    elif variant == "leF32":
        return "leF32"
    elif variant == "leF64":
        return "leF64"
    elif variant == "gtF32":
        return "gtF32"
    elif variant == "gtF64":
        return "gtF64"
    elif variant == "geF32":
        return "geF32"
    elif variant == "geF64":
        return "geF64"
    elif variant == "negI32":
        return "negI32"
    elif variant == "negI64":
        return "negI64"
    elif variant == "not32":
        return "not32"
    elif variant == "not64":
        return "not64"
    elif variant == "negCellInt":
        return "negCellInt"
    elif variant == "notCell":
        return "notCell"
    elif variant == "negWideInt":
        return "negWideInt"
    elif variant == "notWideInt":
        return "notWideInt"
    elif variant == "negF32":
        return "negF32"
    elif variant == "negF64":
        return "negF64"
    elif variant == "unaryFloat":
        return "unaryFloat"
    elif variant == "notBool":
        return "notBool"
    elif variant == "vectorUnary":
        return "vectorUnary"
    elif variant == "tensorUnary":
        return "tensorUnary"
    elif variant == "packedNegI32x4":
        return "packedNegI32x4"
    elif variant == "packedNot32x4":
        return "packedNot32x4"
    elif variant == "packedNegI64x2":
        return "packedNegI64x2"
    elif variant == "packedNot64x2":
        return "packedNot64x2"
    elif variant == "packedNegF32x4":
        return "packedNegF32x4"
    elif variant == "packedNegF64x2":
        return "packedNegF64x2"
    elif variant == "tensorContiguousUnary":
        return "tensorContiguousUnary"
    elif variant == "castBitcast":
        return "castBitcast"
    elif variant == "castTruncate":
        return "castTruncate"
    elif variant == "castZeroExtend":
        return "castZeroExtend"
    elif variant == "castSignExtend":
        return "castSignExtend"
    elif variant == "castFloatToSignedInt":
        return "castFloatToSignedInt"
    elif variant == "castFloatToUnsignedInt":
        return "castFloatToUnsignedInt"
    elif variant == "castFloatToSignedIntSaturating":
        return "castFloatToSignedIntSaturating"
    elif variant == "castFloatToUnsignedIntSaturating":
        return "castFloatToUnsignedIntSaturating"
    elif variant == "castSignedIntToFloat":
        return "castSignedIntToFloat"
    elif variant == "castUnsignedIntToFloat":
        return "castUnsignedIntToFloat"
    elif variant == "castFloatConvert":
        return "castFloatConvert"
    elif variant == "castPointerToInt":
        return "castPointerToInt"
    elif variant == "castIntToPointer":
        return "castIntToPointer"
    elif variant == "castCellToWideInt":
        return "castCellToWideInt"
    elif variant == "castWideIntToCell":
        return "castWideIntToCell"
    elif variant == "castWideInt":
        return "castWideInt"
    elif variant == "castTensorView":
        return "castTensorView"
    elif variant == "call":
        return "call"
    elif variant == "callBranch":
        return "callBranch"
    elif variant == "callFunctionPointer":
        return "callFunctionPointer"
    elif variant == "callFunction":
        return "callFunction"
    elif variant == "callFunctionPointerBranch":
        return "callFunctionPointerBranch"
    elif variant == "callFunctionBranch":
        return "callFunctionBranch"
    elif variant == "callVirtualLocal":
        return "callVirtualLocal"
    elif variant == "callVirtualShared":
        return "callVirtualShared"
    elif variant == "callVirtualLocalBranch":
        return "callVirtualLocalBranch"
    elif variant == "callVirtualSharedBranch":
        return "callVirtualSharedBranch"
    elif variant == "callDynamicLocal":
        return "callDynamicLocal"
    elif variant == "callDynamicShared":
        return "callDynamicShared"
    elif variant == "callDynamicLocalBranch":
        return "callDynamicLocalBranch"
    elif variant == "callDynamicSharedBranch":
        return "callDynamicSharedBranch"
    elif variant == "tailCall":
        return "tailCall"
    elif variant == "tailCallSelf":
        return "tailCallSelf"
    elif variant == "tailCallFunctionPointer":
        return "tailCallFunctionPointer"
    elif variant == "tailCallFunction":
        return "tailCallFunction"
    elif variant == "tailCallVirtualLocal":
        return "tailCallVirtualLocal"
    elif variant == "tailCallVirtualShared":
        return "tailCallVirtualShared"
    elif variant == "tailCallDynamicLocal":
        return "tailCallDynamicLocal"
    elif variant == "tailCallDynamicShared":
        return "tailCallDynamicShared"
    elif variant == "jump":
        return "jump"
    elif variant == "branchBool":
        return "branchBool"
    elif variant == "branchEq32":
        return "branchEq32"
    elif variant == "branchEq64":
        return "branchEq64"
    elif variant == "branchNe32":
        return "branchNe32"
    elif variant == "branchNe64":
        return "branchNe64"
    elif variant == "branchLtI32":
        return "branchLtI32"
    elif variant == "branchLtU32":
        return "branchLtU32"
    elif variant == "branchLtI64":
        return "branchLtI64"
    elif variant == "branchLtU64":
        return "branchLtU64"
    elif variant == "branchLeI32":
        return "branchLeI32"
    elif variant == "branchLeU32":
        return "branchLeU32"
    elif variant == "branchLeI64":
        return "branchLeI64"
    elif variant == "branchLeU64":
        return "branchLeU64"
    elif variant == "branchGtI32":
        return "branchGtI32"
    elif variant == "branchGtU32":
        return "branchGtU32"
    elif variant == "branchGtI64":
        return "branchGtI64"
    elif variant == "branchGtU64":
        return "branchGtU64"
    elif variant == "branchGeI32":
        return "branchGeI32"
    elif variant == "branchGeU32":
        return "branchGeU32"
    elif variant == "branchGeI64":
        return "branchGeI64"
    elif variant == "branchGeU64":
        return "branchGeU64"
    elif variant == "branchEqCell":
        return "branchEqCell"
    elif variant == "branchNeCell":
        return "branchNeCell"
    elif variant == "branchLtCellInt":
        return "branchLtCellInt"
    elif variant == "branchLtCellUint":
        return "branchLtCellUint"
    elif variant == "branchLeCellInt":
        return "branchLeCellInt"
    elif variant == "branchLeCellUint":
        return "branchLeCellUint"
    elif variant == "branchGtCellInt":
        return "branchGtCellInt"
    elif variant == "branchGtCellUint":
        return "branchGtCellUint"
    elif variant == "branchGeCellInt":
        return "branchGeCellInt"
    elif variant == "branchGeCellUint":
        return "branchGeCellUint"
    elif variant == "branchEqF32":
        return "branchEqF32"
    elif variant == "branchEqF64":
        return "branchEqF64"
    elif variant == "branchNeF32":
        return "branchNeF32"
    elif variant == "branchNeF64":
        return "branchNeF64"
    elif variant == "branchLtF32":
        return "branchLtF32"
    elif variant == "branchLtF64":
        return "branchLtF64"
    elif variant == "branchLeF32":
        return "branchLeF32"
    elif variant == "branchLeF64":
        return "branchLeF64"
    elif variant == "branchGtF32":
        return "branchGtF32"
    elif variant == "branchGtF64":
        return "branchGtF64"
    elif variant == "branchGeF32":
        return "branchGeF32"
    elif variant == "branchGeF64":
        return "branchGeF64"
    elif variant == "switch":
        return "switch"
    elif variant == "switchTable":
        return "switchTable"
    elif variant == "check":
        return "check"
    elif variant == "assume":
        return "assume"
    elif variant == "returnCell":
        return "returnCell"
    elif variant == "returnAddress":
        return "returnAddress"
    elif variant == "returnVoid":
        return "returnVoid"
    elif variant == "yieldCell":
        return "yieldCell"
    elif variant == "yieldAddress":
        return "yieldAddress"
    elif variant == "abort":
        return "abort"
    elif variant == "panic":
        return "panic"
    elif variant == "panicValue":
        return "panicValue"
    elif variant == "unwindResume":
        return "unwindResume"
    elif variant == "unreachable":
        return "unreachable"
    elif variant == "barrierWriteHeap":
        return "barrierWriteHeap"
    elif variant == "barrierWriteSharedHeap":
        return "barrierWriteSharedHeap"
    elif variant == "atomicLoad":
        return "atomicLoad"
    elif variant == "atomicStore":
        return "atomicStore"
    elif variant == "atomicExchange":
        return "atomicExchange"
    elif variant == "atomicCompareExchange":
        return "atomicCompareExchange"
    elif variant == "atomicReadModifyWrite":
        return "atomicReadModifyWrite"
    elif variant == "atomicFence":
        return "atomicFence"
    elif variant == "intrinsic":
        return "intrinsic"
    elif variant == "vectorSplat":
        return "vectorSplat"
    elif variant == "packedSplat32x4":
        return "packedSplat32x4"
    elif variant == "packedSplat64x2":
        return "packedSplat64x2"
    elif variant == "vectorExtract":
        return "vectorExtract"
    elif variant == "vectorInsert":
        return "vectorInsert"
    elif variant == "vectorShuffle":
        return "vectorShuffle"
    elif variant == "vectorSelect":
        return "vectorSelect"
    elif variant == "vectorReduce":
        return "vectorReduce"
    elif variant == "vectorConvert":
        return "vectorConvert"
    elif variant == "tensorSplat":
        return "tensorSplat"
    elif variant == "tensorLoad":
        return "tensorLoad"
    elif variant == "tensorExtract":
        return "tensorExtract"
    elif variant == "tensorStore":
        return "tensorStore"
    elif variant == "tensorFill":
        return "tensorFill"
    elif variant == "tensorCopy":
        return "tensorCopy"
    elif variant == "tensorReshape":
        return "tensorReshape"
    elif variant == "tensorBroadcast":
        return "tensorBroadcast"
    elif variant == "tensorTranspose":
        return "tensorTranspose"
    elif variant == "tensorSlice":
        return "tensorSlice"
    elif variant == "tensorPad":
        return "tensorPad"
    elif variant == "tensorConcat":
        return "tensorConcat"
    elif variant == "tensorReduce":
        return "tensorReduce"
    elif variant == "tensorIndexReduce":
        return "tensorIndexReduce"
    elif variant == "tensorDot":
        return "tensorDot"
    elif variant == "tensorConvolution":
        return "tensorConvolution"
    elif variant == "tensorGather":
        return "tensorGather"
    elif variant == "tensorScatter":
        return "tensorScatter"
    elif variant == "tensorSelect":
        return "tensorSelect"
    elif variant == "tensorConvert":
        return "tensorConvert"
    elif variant == "tensorCast":
        return "tensorCast"
    elif variant == "tensorView":
        return "tensorView"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Op",
    "encode_op",
    "decode_op",
    "to_json_op",
    "from_json_op",
]
