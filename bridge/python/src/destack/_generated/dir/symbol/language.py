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

"""Language library items that the toolchain references."""
LanguageItem: typing.TypeAlias = (
    typing.Literal["accessibilityBinding"]
    | typing.Literal["asyncGenerator"]
    | typing.Literal["generator"]
    | typing.Literal["generatorResult"]
    | typing.Literal["generatorReturn"]
    | typing.Literal["generatorState"]
    | typing.Literal["generatorYield"]
    | typing.Literal["asyncIterable"]
    | typing.Literal["asyncIterator"]
    | typing.Literal["continuationHandle"]
    | typing.Literal["continuationResult"]
    | typing.Literal["continuationReturn"]
    | typing.Literal["continuationYield"]
    | typing.Literal["queueMicrotask"]
    | typing.Literal["suspendContinuation"]
    | typing.Literal["promise"]
    | typing.Literal["promiseResolvers"]
    | typing.Literal["audioBinding"]
    | typing.Literal["array"]
    | typing.Literal["fixedArray"]
    | typing.Literal["readonlyArray"]
    | typing.Literal["map"]
    | typing.Literal["set"]
    | typing.Literal["slice"]
    | typing.Literal["computeBuffer"]
    | typing.Literal["computeDevice"]
    | typing.Literal["computeMesh"]
    | typing.Literal["computeKernel"]
    | typing.Literal["computeKernelArgument"]
    | typing.Literal["computeProgram"]
    | typing.Literal["computeEvent"]
    | typing.Literal["computeStream"]
    | typing.Literal["context"]
    | typing.Literal["currentContextValue"]
    | typing.Literal["contextKey"]
    | typing.Literal["contextPatch"]
    | typing.Literal["contextToken"]
    | typing.Literal["contextEntry"]
    | typing.Literal["currentContext"]
    | typing.Literal["getContextValue"]
    | typing.Literal["popContext"]
    | typing.Literal["pushContext"]
    | typing.Literal["requireContextValue"]
    | typing.Literal["as"]
    | typing.Literal["borrow"]
    | typing.Literal["toOwned"]
    | typing.Literal["from"]
    | typing.Literal["tryFrom"]
    | typing.Literal["into"]
    | typing.Literal["tryInto"]
    | typing.Literal["cryptoBinding"]
    | typing.Literal["capture"]
    | typing.Literal["cloneDerive"]
    | typing.Literal["debugDerive"]
    | typing.Literal["tagged"]
    | typing.Literal["allow"]
    | typing.Literal["deny"]
    | typing.Literal["expect"]
    | typing.Literal["forbid"]
    | typing.Literal["warn"]
    | typing.Literal["extern"]
    | typing.Literal["intrinsic"]
    | typing.Literal["languageItem"]
    | typing.Literal["reprDecorator"]
    | typing.Literal["noAliasingMutableBorrows"]
    | typing.Literal["noDynamicDispatch"]
    | typing.Literal["noHeap"]
    | typing.Literal["noImplicitReceivers"]
    | typing.Literal["noManaged"]
    | typing.Literal["noReflection"]
    | typing.Literal["noRuntime"]
    | typing.Literal["noUnsafe"]
    | typing.Literal["noUnwind"]
    | typing.Literal["deprecated"]
    | typing.Literal["experimental"]
    | typing.Literal["cold"]
    | typing.Literal["hot"]
    | typing.Literal["inline"]
    | typing.Literal["likely"]
    | typing.Literal["mustUse"]
    | typing.Literal["noinline"]
    | typing.Literal["pure"]
    | typing.Literal["tailcall"]
    | typing.Literal["unlikely"]
    | typing.Literal["unroll"]
    | typing.Literal["safe"]
    | typing.Literal["sink"]
    | typing.Literal["source"]
    | typing.Literal["taint"]
    | typing.Literal["unsafe"]
    | typing.Literal["untaint"]
    | typing.Literal["deviceBinding"]
    | typing.Literal["displayBinding"]
    | typing.Literal["error"]
    | typing.Literal["abort"]
    | typing.Literal["panic"]
    | typing.Literal["panicValue"]
    | typing.Literal["setPanicHook"]
    | typing.Literal["takePanicHook"]
    | typing.Literal["todo"]
    | typing.Literal["unreachable"]
    | typing.Literal["asyncResult"]
    | typing.Literal["err"]
    | typing.Literal["ok"]
    | typing.Literal["result"]
    | typing.Literal["fsBinding"]
    | typing.Literal["gpuBinding"]
    | typing.Literal["inputBinding"]
    | typing.Literal["ioBinding"]
    | typing.Literal["ipcBinding"]
    | typing.Literal["extend"]
    | typing.Literal["fromIterator"]
    | typing.Literal["iterable"]
    | typing.Literal["iterator"]
    | typing.Literal["eval"]
    | typing.Literal["expansionContext"]
    | typing.Literal["macro"]
    | typing.Literal["macroContext"]
    | typing.Literal["materializationContext"]
    | typing.Literal["bigInt"]
    | typing.Literal["complex"]
    | typing.Literal["math"]
    | typing.Literal["number"]
    | typing.Literal["vector"]
    | typing.Literal["access"]
    | typing.Literal["dynamic"]
    | typing.Literal["allocationError"]
    | typing.Literal["memoryBinding"]
    | typing.Literal["arc"]
    | typing.Literal["arcInner"]
    | typing.Literal["arcWeak"]
    | typing.Literal["borrowed"]
    | typing.Literal["box"]
    | typing.Literal["clone"]
    | typing.Literal["concrete"]
    | typing.Literal["copy"]
    | typing.Literal["default"]
    | typing.Literal["dynamicSafe"]
    | typing.Literal["send"]
    | typing.Literal["sync"]
    | typing.Literal["unpin"]
    | typing.Literal["zeroable"]
    | typing.Literal["unsafeCell"]
    | typing.Literal["asyncDispose"]
    | typing.Literal["dispose"]
    | typing.Literal["drop"]
    | typing.Literal["forget"]
    | typing.Literal["manuallyDrop"]
    | typing.Literal["maybeUninit"]
    | typing.Literal["lifetime"]
    | typing.Literal["managed"]
    | typing.Literal["owned"]
    | typing.Literal["phantom"]
    | typing.Literal["pin"]
    | typing.Literal["place"]
    | typing.Literal["placed"]
    | typing.Literal["space"]
    | typing.Literal["raw"]
    | typing.Literal["rc"]
    | typing.Literal["rcInner"]
    | typing.Literal["rcWeak"]
    | typing.Literal["accessOf"]
    | typing.Literal["accessOr"]
    | typing.Literal["baseOf"]
    | typing.Literal["isBorrowed"]
    | typing.Literal["isManaged"]
    | typing.Literal["isOwned"]
    | typing.Literal["isRaw"]
    | typing.Literal["isShared"]
    | typing.Literal["isSharedIn"]
    | typing.Literal["lifetimeOf"]
    | typing.Literal["lifetimeOr"]
    | typing.Literal["ownership"]
    | typing.Literal["ownershipOf"]
    | typing.Literal["ownershipOr"]
    | typing.Literal["payloadOf"]
    | typing.Literal["placeIn"]
    | typing.Literal["placeOf"]
    | typing.Literal["placeOr"]
    | typing.Literal["spaceOf"]
    | typing.Literal["spaceOr"]
    | typing.Literal["withAccess"]
    | typing.Literal["withBase"]
    | typing.Literal["withLifetime"]
    | typing.Literal["withOwnership"]
    | typing.Literal["withPlace"]
    | typing.Literal["withSpace"]
    | typing.Literal["unique"]
    | typing.Literal["importMeta"]
    | typing.Literal["importMetaEnv"]
    | typing.Literal["netBinding"]
    | typing.Literal["and"]
    | typing.Literal["not"]
    | typing.Literal["or"]
    | typing.Literal["xor"]
    | typing.Literal["compare"]
    | typing.Literal["ordering"]
    | typing.Literal["partialCompare"]
    | typing.Literal["dereference"]
    | typing.Literal["divide"]
    | typing.Literal["equal"]
    | typing.Literal["partialEqual"]
    | typing.Literal["debug"]
    | typing.Literal["display"]
    | typing.Literal["hash"]
    | typing.Literal["hasher"]
    | typing.Literal["subtract"]
    | typing.Literal["multiply"]
    | typing.Literal["negate"]
    | typing.Literal["add"]
    | typing.Literal["plus"]
    | typing.Literal["power"]
    | typing.Literal["remainder"]
    | typing.Literal["shiftLeft"]
    | typing.Literal["shiftRight"]
    | typing.Literal["shiftRightUnsigned"]
    | typing.Literal["index"]
    | typing.Literal["indexSet"]
    | typing.Literal["fromResidual"]
    | typing.Literal["try"]
    | typing.Literal["controlFlow"]
    | typing.Literal["osBinding"]
    | typing.Literal["processBinding"]
    | typing.Literal["randomBinding"]
    | typing.Literal["bound"]
    | typing.Literal["rangeBounds"]
    | typing.Literal["range"]
    | typing.Literal["rangeFrom"]
    | typing.Literal["rangeFull"]
    | typing.Literal["rangeInclusive"]
    | typing.Literal["rangeTo"]
    | typing.Literal["rangeToInclusive"]
    | typing.Literal["step"]
    | typing.Literal["reflect"]
    | typing.Literal["alignOf"]
    | typing.Literal["layout"]
    | typing.Literal["layoutField"]
    | typing.Literal["layoutOf"]
    | typing.Literal["layoutShape"]
    | typing.Literal["layoutVariant"]
    | typing.Literal["sizeOf"]
    | typing.Literal["strideOf"]
    | typing.Literal["type"]
    | typing.Literal["typeId"]
    | typing.Literal["typeOf"]
    | typing.Literal["regExp"]
    | typing.Literal["binding"]
    | typing.Literal["deserialize"]
    | typing.Literal["deserializer"]
    | typing.Literal["serialize"]
    | typing.Literal["serializer"]
    | typing.Literal["stringSlice"]
    | typing.Literal["string"]
    | typing.Literal["tensorFormat"]
    | typing.Literal["tensorViewFormat"]
    | typing.Literal["tensorDense"]
    | typing.Literal["tensorStrided"]
    | typing.Literal["tensorShape"]
    | typing.Literal["tensorPlacement"]
    | typing.Literal["tensorShardingAxis"]
    | typing.Literal["tensorUnsharded"]
    | typing.Literal["tensorShardingAxes"]
    | typing.Literal["tensorShard"]
    | typing.Literal["tensorReplicate"]
    | typing.Literal["tensorPartial"]
    | typing.Literal["tensor"]
    | typing.Literal["tensorView"]
    | typing.Literal["timeBinding"]
    | typing.Literal["topologyBinding"]
    | typing.Literal["tlsBinding"]
    | typing.Literal["ttyBinding"]
    | typing.Literal["constructorParameters"]
    | typing.Literal["function"]
    | typing.Literal["functionPointer"]
    | typing.Literal["instanceType"]
    | typing.Literal["omitThisParameter"]
    | typing.Literal["parameters"]
    | typing.Literal["returnType"]
    | typing.Literal["thisParameterType"]
    | typing.Literal["awaited"]
    | typing.Literal["exclude"]
    | typing.Literal["extract"]
    | typing.Literal["nonNullable"]
    | typing.Literal["noInfer"]
    | typing.Literal["omit"]
    | typing.Literal["partial"]
    | typing.Literal["pick"]
    | typing.Literal["propertyKey"]
    | typing.Literal["readonly"]
    | typing.Literal["record"]
    | typing.Literal["required"]
    | typing.Literal["thisType"]
    | typing.Literal["capitalize"]
    | typing.Literal["lowercase"]
    | typing.Literal["uncapitalize"]
    | typing.Literal["uppercase"]
    | typing.Literal["option"]
    | typing.Literal["symbol"]
)


def encode_language_item(writer: BinaryWriter, value: LanguageItem) -> None:
    """Encode one LanguageItem."""
    if value == "accessibilityBinding":
        writer.write_unsigned(0)
    elif value == "asyncGenerator":
        writer.write_unsigned(1)
    elif value == "generator":
        writer.write_unsigned(2)
    elif value == "generatorResult":
        writer.write_unsigned(3)
    elif value == "generatorReturn":
        writer.write_unsigned(4)
    elif value == "generatorState":
        writer.write_unsigned(5)
    elif value == "generatorYield":
        writer.write_unsigned(6)
    elif value == "asyncIterable":
        writer.write_unsigned(7)
    elif value == "asyncIterator":
        writer.write_unsigned(8)
    elif value == "continuationHandle":
        writer.write_unsigned(9)
    elif value == "continuationResult":
        writer.write_unsigned(10)
    elif value == "continuationReturn":
        writer.write_unsigned(11)
    elif value == "continuationYield":
        writer.write_unsigned(12)
    elif value == "queueMicrotask":
        writer.write_unsigned(13)
    elif value == "suspendContinuation":
        writer.write_unsigned(14)
    elif value == "promise":
        writer.write_unsigned(15)
    elif value == "promiseResolvers":
        writer.write_unsigned(16)
    elif value == "audioBinding":
        writer.write_unsigned(17)
    elif value == "array":
        writer.write_unsigned(18)
    elif value == "fixedArray":
        writer.write_unsigned(19)
    elif value == "readonlyArray":
        writer.write_unsigned(20)
    elif value == "map":
        writer.write_unsigned(21)
    elif value == "set":
        writer.write_unsigned(22)
    elif value == "slice":
        writer.write_unsigned(23)
    elif value == "computeBuffer":
        writer.write_unsigned(24)
    elif value == "computeDevice":
        writer.write_unsigned(25)
    elif value == "computeMesh":
        writer.write_unsigned(26)
    elif value == "computeKernel":
        writer.write_unsigned(27)
    elif value == "computeKernelArgument":
        writer.write_unsigned(28)
    elif value == "computeProgram":
        writer.write_unsigned(29)
    elif value == "computeEvent":
        writer.write_unsigned(30)
    elif value == "computeStream":
        writer.write_unsigned(31)
    elif value == "context":
        writer.write_unsigned(32)
    elif value == "currentContextValue":
        writer.write_unsigned(33)
    elif value == "contextKey":
        writer.write_unsigned(34)
    elif value == "contextPatch":
        writer.write_unsigned(35)
    elif value == "contextToken":
        writer.write_unsigned(36)
    elif value == "contextEntry":
        writer.write_unsigned(37)
    elif value == "currentContext":
        writer.write_unsigned(38)
    elif value == "getContextValue":
        writer.write_unsigned(39)
    elif value == "popContext":
        writer.write_unsigned(40)
    elif value == "pushContext":
        writer.write_unsigned(41)
    elif value == "requireContextValue":
        writer.write_unsigned(42)
    elif value == "as":
        writer.write_unsigned(43)
    elif value == "borrow":
        writer.write_unsigned(44)
    elif value == "toOwned":
        writer.write_unsigned(45)
    elif value == "from":
        writer.write_unsigned(46)
    elif value == "tryFrom":
        writer.write_unsigned(47)
    elif value == "into":
        writer.write_unsigned(48)
    elif value == "tryInto":
        writer.write_unsigned(49)
    elif value == "cryptoBinding":
        writer.write_unsigned(50)
    elif value == "capture":
        writer.write_unsigned(51)
    elif value == "cloneDerive":
        writer.write_unsigned(52)
    elif value == "debugDerive":
        writer.write_unsigned(53)
    elif value == "tagged":
        writer.write_unsigned(54)
    elif value == "allow":
        writer.write_unsigned(55)
    elif value == "deny":
        writer.write_unsigned(56)
    elif value == "expect":
        writer.write_unsigned(57)
    elif value == "forbid":
        writer.write_unsigned(58)
    elif value == "warn":
        writer.write_unsigned(59)
    elif value == "extern":
        writer.write_unsigned(60)
    elif value == "intrinsic":
        writer.write_unsigned(61)
    elif value == "languageItem":
        writer.write_unsigned(62)
    elif value == "reprDecorator":
        writer.write_unsigned(63)
    elif value == "noAliasingMutableBorrows":
        writer.write_unsigned(64)
    elif value == "noDynamicDispatch":
        writer.write_unsigned(65)
    elif value == "noHeap":
        writer.write_unsigned(66)
    elif value == "noImplicitReceivers":
        writer.write_unsigned(67)
    elif value == "noManaged":
        writer.write_unsigned(68)
    elif value == "noReflection":
        writer.write_unsigned(69)
    elif value == "noRuntime":
        writer.write_unsigned(70)
    elif value == "noUnsafe":
        writer.write_unsigned(71)
    elif value == "noUnwind":
        writer.write_unsigned(72)
    elif value == "deprecated":
        writer.write_unsigned(73)
    elif value == "experimental":
        writer.write_unsigned(74)
    elif value == "cold":
        writer.write_unsigned(75)
    elif value == "hot":
        writer.write_unsigned(76)
    elif value == "inline":
        writer.write_unsigned(77)
    elif value == "likely":
        writer.write_unsigned(78)
    elif value == "mustUse":
        writer.write_unsigned(79)
    elif value == "noinline":
        writer.write_unsigned(80)
    elif value == "pure":
        writer.write_unsigned(81)
    elif value == "tailcall":
        writer.write_unsigned(82)
    elif value == "unlikely":
        writer.write_unsigned(83)
    elif value == "unroll":
        writer.write_unsigned(84)
    elif value == "safe":
        writer.write_unsigned(85)
    elif value == "sink":
        writer.write_unsigned(86)
    elif value == "source":
        writer.write_unsigned(87)
    elif value == "taint":
        writer.write_unsigned(88)
    elif value == "unsafe":
        writer.write_unsigned(89)
    elif value == "untaint":
        writer.write_unsigned(90)
    elif value == "deviceBinding":
        writer.write_unsigned(91)
    elif value == "displayBinding":
        writer.write_unsigned(92)
    elif value == "error":
        writer.write_unsigned(93)
    elif value == "abort":
        writer.write_unsigned(94)
    elif value == "panic":
        writer.write_unsigned(95)
    elif value == "panicValue":
        writer.write_unsigned(96)
    elif value == "setPanicHook":
        writer.write_unsigned(97)
    elif value == "takePanicHook":
        writer.write_unsigned(98)
    elif value == "todo":
        writer.write_unsigned(99)
    elif value == "unreachable":
        writer.write_unsigned(100)
    elif value == "asyncResult":
        writer.write_unsigned(101)
    elif value == "err":
        writer.write_unsigned(102)
    elif value == "ok":
        writer.write_unsigned(103)
    elif value == "result":
        writer.write_unsigned(104)
    elif value == "fsBinding":
        writer.write_unsigned(105)
    elif value == "gpuBinding":
        writer.write_unsigned(106)
    elif value == "inputBinding":
        writer.write_unsigned(107)
    elif value == "ioBinding":
        writer.write_unsigned(108)
    elif value == "ipcBinding":
        writer.write_unsigned(109)
    elif value == "extend":
        writer.write_unsigned(110)
    elif value == "fromIterator":
        writer.write_unsigned(111)
    elif value == "iterable":
        writer.write_unsigned(112)
    elif value == "iterator":
        writer.write_unsigned(113)
    elif value == "eval":
        writer.write_unsigned(114)
    elif value == "expansionContext":
        writer.write_unsigned(115)
    elif value == "macro":
        writer.write_unsigned(116)
    elif value == "macroContext":
        writer.write_unsigned(117)
    elif value == "materializationContext":
        writer.write_unsigned(118)
    elif value == "bigInt":
        writer.write_unsigned(119)
    elif value == "complex":
        writer.write_unsigned(120)
    elif value == "math":
        writer.write_unsigned(121)
    elif value == "number":
        writer.write_unsigned(122)
    elif value == "vector":
        writer.write_unsigned(123)
    elif value == "access":
        writer.write_unsigned(124)
    elif value == "dynamic":
        writer.write_unsigned(125)
    elif value == "allocationError":
        writer.write_unsigned(126)
    elif value == "memoryBinding":
        writer.write_unsigned(127)
    elif value == "arc":
        writer.write_unsigned(128)
    elif value == "arcInner":
        writer.write_unsigned(129)
    elif value == "arcWeak":
        writer.write_unsigned(130)
    elif value == "borrowed":
        writer.write_unsigned(131)
    elif value == "box":
        writer.write_unsigned(132)
    elif value == "clone":
        writer.write_unsigned(133)
    elif value == "concrete":
        writer.write_unsigned(134)
    elif value == "copy":
        writer.write_unsigned(135)
    elif value == "default":
        writer.write_unsigned(136)
    elif value == "dynamicSafe":
        writer.write_unsigned(137)
    elif value == "send":
        writer.write_unsigned(138)
    elif value == "sync":
        writer.write_unsigned(139)
    elif value == "unpin":
        writer.write_unsigned(140)
    elif value == "zeroable":
        writer.write_unsigned(141)
    elif value == "unsafeCell":
        writer.write_unsigned(142)
    elif value == "asyncDispose":
        writer.write_unsigned(143)
    elif value == "dispose":
        writer.write_unsigned(144)
    elif value == "drop":
        writer.write_unsigned(145)
    elif value == "forget":
        writer.write_unsigned(146)
    elif value == "manuallyDrop":
        writer.write_unsigned(147)
    elif value == "maybeUninit":
        writer.write_unsigned(148)
    elif value == "lifetime":
        writer.write_unsigned(149)
    elif value == "managed":
        writer.write_unsigned(150)
    elif value == "owned":
        writer.write_unsigned(151)
    elif value == "phantom":
        writer.write_unsigned(152)
    elif value == "pin":
        writer.write_unsigned(153)
    elif value == "place":
        writer.write_unsigned(154)
    elif value == "placed":
        writer.write_unsigned(155)
    elif value == "space":
        writer.write_unsigned(156)
    elif value == "raw":
        writer.write_unsigned(157)
    elif value == "rc":
        writer.write_unsigned(158)
    elif value == "rcInner":
        writer.write_unsigned(159)
    elif value == "rcWeak":
        writer.write_unsigned(160)
    elif value == "accessOf":
        writer.write_unsigned(161)
    elif value == "accessOr":
        writer.write_unsigned(162)
    elif value == "baseOf":
        writer.write_unsigned(163)
    elif value == "isBorrowed":
        writer.write_unsigned(164)
    elif value == "isManaged":
        writer.write_unsigned(165)
    elif value == "isOwned":
        writer.write_unsigned(166)
    elif value == "isRaw":
        writer.write_unsigned(167)
    elif value == "isShared":
        writer.write_unsigned(168)
    elif value == "isSharedIn":
        writer.write_unsigned(169)
    elif value == "lifetimeOf":
        writer.write_unsigned(170)
    elif value == "lifetimeOr":
        writer.write_unsigned(171)
    elif value == "ownership":
        writer.write_unsigned(172)
    elif value == "ownershipOf":
        writer.write_unsigned(173)
    elif value == "ownershipOr":
        writer.write_unsigned(174)
    elif value == "payloadOf":
        writer.write_unsigned(175)
    elif value == "placeIn":
        writer.write_unsigned(176)
    elif value == "placeOf":
        writer.write_unsigned(177)
    elif value == "placeOr":
        writer.write_unsigned(178)
    elif value == "spaceOf":
        writer.write_unsigned(179)
    elif value == "spaceOr":
        writer.write_unsigned(180)
    elif value == "withAccess":
        writer.write_unsigned(181)
    elif value == "withBase":
        writer.write_unsigned(182)
    elif value == "withLifetime":
        writer.write_unsigned(183)
    elif value == "withOwnership":
        writer.write_unsigned(184)
    elif value == "withPlace":
        writer.write_unsigned(185)
    elif value == "withSpace":
        writer.write_unsigned(186)
    elif value == "unique":
        writer.write_unsigned(187)
    elif value == "importMeta":
        writer.write_unsigned(188)
    elif value == "importMetaEnv":
        writer.write_unsigned(189)
    elif value == "netBinding":
        writer.write_unsigned(190)
    elif value == "and":
        writer.write_unsigned(191)
    elif value == "not":
        writer.write_unsigned(192)
    elif value == "or":
        writer.write_unsigned(193)
    elif value == "xor":
        writer.write_unsigned(194)
    elif value == "compare":
        writer.write_unsigned(195)
    elif value == "ordering":
        writer.write_unsigned(196)
    elif value == "partialCompare":
        writer.write_unsigned(197)
    elif value == "dereference":
        writer.write_unsigned(198)
    elif value == "divide":
        writer.write_unsigned(199)
    elif value == "equal":
        writer.write_unsigned(200)
    elif value == "partialEqual":
        writer.write_unsigned(201)
    elif value == "debug":
        writer.write_unsigned(202)
    elif value == "display":
        writer.write_unsigned(203)
    elif value == "hash":
        writer.write_unsigned(204)
    elif value == "hasher":
        writer.write_unsigned(205)
    elif value == "subtract":
        writer.write_unsigned(206)
    elif value == "multiply":
        writer.write_unsigned(207)
    elif value == "negate":
        writer.write_unsigned(208)
    elif value == "add":
        writer.write_unsigned(209)
    elif value == "plus":
        writer.write_unsigned(210)
    elif value == "power":
        writer.write_unsigned(211)
    elif value == "remainder":
        writer.write_unsigned(212)
    elif value == "shiftLeft":
        writer.write_unsigned(213)
    elif value == "shiftRight":
        writer.write_unsigned(214)
    elif value == "shiftRightUnsigned":
        writer.write_unsigned(215)
    elif value == "index":
        writer.write_unsigned(216)
    elif value == "indexSet":
        writer.write_unsigned(217)
    elif value == "fromResidual":
        writer.write_unsigned(218)
    elif value == "try":
        writer.write_unsigned(219)
    elif value == "controlFlow":
        writer.write_unsigned(220)
    elif value == "osBinding":
        writer.write_unsigned(221)
    elif value == "processBinding":
        writer.write_unsigned(222)
    elif value == "randomBinding":
        writer.write_unsigned(223)
    elif value == "bound":
        writer.write_unsigned(224)
    elif value == "rangeBounds":
        writer.write_unsigned(225)
    elif value == "range":
        writer.write_unsigned(226)
    elif value == "rangeFrom":
        writer.write_unsigned(227)
    elif value == "rangeFull":
        writer.write_unsigned(228)
    elif value == "rangeInclusive":
        writer.write_unsigned(229)
    elif value == "rangeTo":
        writer.write_unsigned(230)
    elif value == "rangeToInclusive":
        writer.write_unsigned(231)
    elif value == "step":
        writer.write_unsigned(232)
    elif value == "reflect":
        writer.write_unsigned(233)
    elif value == "alignOf":
        writer.write_unsigned(234)
    elif value == "layout":
        writer.write_unsigned(235)
    elif value == "layoutField":
        writer.write_unsigned(236)
    elif value == "layoutOf":
        writer.write_unsigned(237)
    elif value == "layoutShape":
        writer.write_unsigned(238)
    elif value == "layoutVariant":
        writer.write_unsigned(239)
    elif value == "sizeOf":
        writer.write_unsigned(240)
    elif value == "strideOf":
        writer.write_unsigned(241)
    elif value == "type":
        writer.write_unsigned(242)
    elif value == "typeId":
        writer.write_unsigned(243)
    elif value == "typeOf":
        writer.write_unsigned(244)
    elif value == "regExp":
        writer.write_unsigned(245)
    elif value == "binding":
        writer.write_unsigned(246)
    elif value == "deserialize":
        writer.write_unsigned(247)
    elif value == "deserializer":
        writer.write_unsigned(248)
    elif value == "serialize":
        writer.write_unsigned(249)
    elif value == "serializer":
        writer.write_unsigned(250)
    elif value == "stringSlice":
        writer.write_unsigned(251)
    elif value == "string":
        writer.write_unsigned(252)
    elif value == "tensorFormat":
        writer.write_unsigned(253)
    elif value == "tensorViewFormat":
        writer.write_unsigned(254)
    elif value == "tensorDense":
        writer.write_unsigned(255)
    elif value == "tensorStrided":
        writer.write_unsigned(256)
    elif value == "tensorShape":
        writer.write_unsigned(257)
    elif value == "tensorPlacement":
        writer.write_unsigned(258)
    elif value == "tensorShardingAxis":
        writer.write_unsigned(259)
    elif value == "tensorUnsharded":
        writer.write_unsigned(260)
    elif value == "tensorShardingAxes":
        writer.write_unsigned(261)
    elif value == "tensorShard":
        writer.write_unsigned(262)
    elif value == "tensorReplicate":
        writer.write_unsigned(263)
    elif value == "tensorPartial":
        writer.write_unsigned(264)
    elif value == "tensor":
        writer.write_unsigned(265)
    elif value == "tensorView":
        writer.write_unsigned(266)
    elif value == "timeBinding":
        writer.write_unsigned(267)
    elif value == "topologyBinding":
        writer.write_unsigned(268)
    elif value == "tlsBinding":
        writer.write_unsigned(269)
    elif value == "ttyBinding":
        writer.write_unsigned(270)
    elif value == "constructorParameters":
        writer.write_unsigned(271)
    elif value == "function":
        writer.write_unsigned(272)
    elif value == "functionPointer":
        writer.write_unsigned(273)
    elif value == "instanceType":
        writer.write_unsigned(274)
    elif value == "omitThisParameter":
        writer.write_unsigned(275)
    elif value == "parameters":
        writer.write_unsigned(276)
    elif value == "returnType":
        writer.write_unsigned(277)
    elif value == "thisParameterType":
        writer.write_unsigned(278)
    elif value == "awaited":
        writer.write_unsigned(279)
    elif value == "exclude":
        writer.write_unsigned(280)
    elif value == "extract":
        writer.write_unsigned(281)
    elif value == "nonNullable":
        writer.write_unsigned(282)
    elif value == "noInfer":
        writer.write_unsigned(283)
    elif value == "omit":
        writer.write_unsigned(284)
    elif value == "partial":
        writer.write_unsigned(285)
    elif value == "pick":
        writer.write_unsigned(286)
    elif value == "propertyKey":
        writer.write_unsigned(287)
    elif value == "readonly":
        writer.write_unsigned(288)
    elif value == "record":
        writer.write_unsigned(289)
    elif value == "required":
        writer.write_unsigned(290)
    elif value == "thisType":
        writer.write_unsigned(291)
    elif value == "capitalize":
        writer.write_unsigned(292)
    elif value == "lowercase":
        writer.write_unsigned(293)
    elif value == "uncapitalize":
        writer.write_unsigned(294)
    elif value == "uppercase":
        writer.write_unsigned(295)
    elif value == "option":
        writer.write_unsigned(296)
    elif value == "symbol":
        writer.write_unsigned(297)
    else:
        raise SerdeError("unknown enum variant")


def decode_language_item(reader: BinaryReader) -> LanguageItem:
    """Decode one LanguageItem."""
    variant = reader.read_number()

    if variant == 0:
        return "accessibilityBinding"
    elif variant == 1:
        return "asyncGenerator"
    elif variant == 2:
        return "generator"
    elif variant == 3:
        return "generatorResult"
    elif variant == 4:
        return "generatorReturn"
    elif variant == 5:
        return "generatorState"
    elif variant == 6:
        return "generatorYield"
    elif variant == 7:
        return "asyncIterable"
    elif variant == 8:
        return "asyncIterator"
    elif variant == 9:
        return "continuationHandle"
    elif variant == 10:
        return "continuationResult"
    elif variant == 11:
        return "continuationReturn"
    elif variant == 12:
        return "continuationYield"
    elif variant == 13:
        return "queueMicrotask"
    elif variant == 14:
        return "suspendContinuation"
    elif variant == 15:
        return "promise"
    elif variant == 16:
        return "promiseResolvers"
    elif variant == 17:
        return "audioBinding"
    elif variant == 18:
        return "array"
    elif variant == 19:
        return "fixedArray"
    elif variant == 20:
        return "readonlyArray"
    elif variant == 21:
        return "map"
    elif variant == 22:
        return "set"
    elif variant == 23:
        return "slice"
    elif variant == 24:
        return "computeBuffer"
    elif variant == 25:
        return "computeDevice"
    elif variant == 26:
        return "computeMesh"
    elif variant == 27:
        return "computeKernel"
    elif variant == 28:
        return "computeKernelArgument"
    elif variant == 29:
        return "computeProgram"
    elif variant == 30:
        return "computeEvent"
    elif variant == 31:
        return "computeStream"
    elif variant == 32:
        return "context"
    elif variant == 33:
        return "currentContextValue"
    elif variant == 34:
        return "contextKey"
    elif variant == 35:
        return "contextPatch"
    elif variant == 36:
        return "contextToken"
    elif variant == 37:
        return "contextEntry"
    elif variant == 38:
        return "currentContext"
    elif variant == 39:
        return "getContextValue"
    elif variant == 40:
        return "popContext"
    elif variant == 41:
        return "pushContext"
    elif variant == 42:
        return "requireContextValue"
    elif variant == 43:
        return "as"
    elif variant == 44:
        return "borrow"
    elif variant == 45:
        return "toOwned"
    elif variant == 46:
        return "from"
    elif variant == 47:
        return "tryFrom"
    elif variant == 48:
        return "into"
    elif variant == 49:
        return "tryInto"
    elif variant == 50:
        return "cryptoBinding"
    elif variant == 51:
        return "capture"
    elif variant == 52:
        return "cloneDerive"
    elif variant == 53:
        return "debugDerive"
    elif variant == 54:
        return "tagged"
    elif variant == 55:
        return "allow"
    elif variant == 56:
        return "deny"
    elif variant == 57:
        return "expect"
    elif variant == 58:
        return "forbid"
    elif variant == 59:
        return "warn"
    elif variant == 60:
        return "extern"
    elif variant == 61:
        return "intrinsic"
    elif variant == 62:
        return "languageItem"
    elif variant == 63:
        return "reprDecorator"
    elif variant == 64:
        return "noAliasingMutableBorrows"
    elif variant == 65:
        return "noDynamicDispatch"
    elif variant == 66:
        return "noHeap"
    elif variant == 67:
        return "noImplicitReceivers"
    elif variant == 68:
        return "noManaged"
    elif variant == 69:
        return "noReflection"
    elif variant == 70:
        return "noRuntime"
    elif variant == 71:
        return "noUnsafe"
    elif variant == 72:
        return "noUnwind"
    elif variant == 73:
        return "deprecated"
    elif variant == 74:
        return "experimental"
    elif variant == 75:
        return "cold"
    elif variant == 76:
        return "hot"
    elif variant == 77:
        return "inline"
    elif variant == 78:
        return "likely"
    elif variant == 79:
        return "mustUse"
    elif variant == 80:
        return "noinline"
    elif variant == 81:
        return "pure"
    elif variant == 82:
        return "tailcall"
    elif variant == 83:
        return "unlikely"
    elif variant == 84:
        return "unroll"
    elif variant == 85:
        return "safe"
    elif variant == 86:
        return "sink"
    elif variant == 87:
        return "source"
    elif variant == 88:
        return "taint"
    elif variant == 89:
        return "unsafe"
    elif variant == 90:
        return "untaint"
    elif variant == 91:
        return "deviceBinding"
    elif variant == 92:
        return "displayBinding"
    elif variant == 93:
        return "error"
    elif variant == 94:
        return "abort"
    elif variant == 95:
        return "panic"
    elif variant == 96:
        return "panicValue"
    elif variant == 97:
        return "setPanicHook"
    elif variant == 98:
        return "takePanicHook"
    elif variant == 99:
        return "todo"
    elif variant == 100:
        return "unreachable"
    elif variant == 101:
        return "asyncResult"
    elif variant == 102:
        return "err"
    elif variant == 103:
        return "ok"
    elif variant == 104:
        return "result"
    elif variant == 105:
        return "fsBinding"
    elif variant == 106:
        return "gpuBinding"
    elif variant == 107:
        return "inputBinding"
    elif variant == 108:
        return "ioBinding"
    elif variant == 109:
        return "ipcBinding"
    elif variant == 110:
        return "extend"
    elif variant == 111:
        return "fromIterator"
    elif variant == 112:
        return "iterable"
    elif variant == 113:
        return "iterator"
    elif variant == 114:
        return "eval"
    elif variant == 115:
        return "expansionContext"
    elif variant == 116:
        return "macro"
    elif variant == 117:
        return "macroContext"
    elif variant == 118:
        return "materializationContext"
    elif variant == 119:
        return "bigInt"
    elif variant == 120:
        return "complex"
    elif variant == 121:
        return "math"
    elif variant == 122:
        return "number"
    elif variant == 123:
        return "vector"
    elif variant == 124:
        return "access"
    elif variant == 125:
        return "dynamic"
    elif variant == 126:
        return "allocationError"
    elif variant == 127:
        return "memoryBinding"
    elif variant == 128:
        return "arc"
    elif variant == 129:
        return "arcInner"
    elif variant == 130:
        return "arcWeak"
    elif variant == 131:
        return "borrowed"
    elif variant == 132:
        return "box"
    elif variant == 133:
        return "clone"
    elif variant == 134:
        return "concrete"
    elif variant == 135:
        return "copy"
    elif variant == 136:
        return "default"
    elif variant == 137:
        return "dynamicSafe"
    elif variant == 138:
        return "send"
    elif variant == 139:
        return "sync"
    elif variant == 140:
        return "unpin"
    elif variant == 141:
        return "zeroable"
    elif variant == 142:
        return "unsafeCell"
    elif variant == 143:
        return "asyncDispose"
    elif variant == 144:
        return "dispose"
    elif variant == 145:
        return "drop"
    elif variant == 146:
        return "forget"
    elif variant == 147:
        return "manuallyDrop"
    elif variant == 148:
        return "maybeUninit"
    elif variant == 149:
        return "lifetime"
    elif variant == 150:
        return "managed"
    elif variant == 151:
        return "owned"
    elif variant == 152:
        return "phantom"
    elif variant == 153:
        return "pin"
    elif variant == 154:
        return "place"
    elif variant == 155:
        return "placed"
    elif variant == 156:
        return "space"
    elif variant == 157:
        return "raw"
    elif variant == 158:
        return "rc"
    elif variant == 159:
        return "rcInner"
    elif variant == 160:
        return "rcWeak"
    elif variant == 161:
        return "accessOf"
    elif variant == 162:
        return "accessOr"
    elif variant == 163:
        return "baseOf"
    elif variant == 164:
        return "isBorrowed"
    elif variant == 165:
        return "isManaged"
    elif variant == 166:
        return "isOwned"
    elif variant == 167:
        return "isRaw"
    elif variant == 168:
        return "isShared"
    elif variant == 169:
        return "isSharedIn"
    elif variant == 170:
        return "lifetimeOf"
    elif variant == 171:
        return "lifetimeOr"
    elif variant == 172:
        return "ownership"
    elif variant == 173:
        return "ownershipOf"
    elif variant == 174:
        return "ownershipOr"
    elif variant == 175:
        return "payloadOf"
    elif variant == 176:
        return "placeIn"
    elif variant == 177:
        return "placeOf"
    elif variant == 178:
        return "placeOr"
    elif variant == 179:
        return "spaceOf"
    elif variant == 180:
        return "spaceOr"
    elif variant == 181:
        return "withAccess"
    elif variant == 182:
        return "withBase"
    elif variant == 183:
        return "withLifetime"
    elif variant == 184:
        return "withOwnership"
    elif variant == 185:
        return "withPlace"
    elif variant == 186:
        return "withSpace"
    elif variant == 187:
        return "unique"
    elif variant == 188:
        return "importMeta"
    elif variant == 189:
        return "importMetaEnv"
    elif variant == 190:
        return "netBinding"
    elif variant == 191:
        return "and"
    elif variant == 192:
        return "not"
    elif variant == 193:
        return "or"
    elif variant == 194:
        return "xor"
    elif variant == 195:
        return "compare"
    elif variant == 196:
        return "ordering"
    elif variant == 197:
        return "partialCompare"
    elif variant == 198:
        return "dereference"
    elif variant == 199:
        return "divide"
    elif variant == 200:
        return "equal"
    elif variant == 201:
        return "partialEqual"
    elif variant == 202:
        return "debug"
    elif variant == 203:
        return "display"
    elif variant == 204:
        return "hash"
    elif variant == 205:
        return "hasher"
    elif variant == 206:
        return "subtract"
    elif variant == 207:
        return "multiply"
    elif variant == 208:
        return "negate"
    elif variant == 209:
        return "add"
    elif variant == 210:
        return "plus"
    elif variant == 211:
        return "power"
    elif variant == 212:
        return "remainder"
    elif variant == 213:
        return "shiftLeft"
    elif variant == 214:
        return "shiftRight"
    elif variant == 215:
        return "shiftRightUnsigned"
    elif variant == 216:
        return "index"
    elif variant == 217:
        return "indexSet"
    elif variant == 218:
        return "fromResidual"
    elif variant == 219:
        return "try"
    elif variant == 220:
        return "controlFlow"
    elif variant == 221:
        return "osBinding"
    elif variant == 222:
        return "processBinding"
    elif variant == 223:
        return "randomBinding"
    elif variant == 224:
        return "bound"
    elif variant == 225:
        return "rangeBounds"
    elif variant == 226:
        return "range"
    elif variant == 227:
        return "rangeFrom"
    elif variant == 228:
        return "rangeFull"
    elif variant == 229:
        return "rangeInclusive"
    elif variant == 230:
        return "rangeTo"
    elif variant == 231:
        return "rangeToInclusive"
    elif variant == 232:
        return "step"
    elif variant == 233:
        return "reflect"
    elif variant == 234:
        return "alignOf"
    elif variant == 235:
        return "layout"
    elif variant == 236:
        return "layoutField"
    elif variant == 237:
        return "layoutOf"
    elif variant == 238:
        return "layoutShape"
    elif variant == 239:
        return "layoutVariant"
    elif variant == 240:
        return "sizeOf"
    elif variant == 241:
        return "strideOf"
    elif variant == 242:
        return "type"
    elif variant == 243:
        return "typeId"
    elif variant == 244:
        return "typeOf"
    elif variant == 245:
        return "regExp"
    elif variant == 246:
        return "binding"
    elif variant == 247:
        return "deserialize"
    elif variant == 248:
        return "deserializer"
    elif variant == 249:
        return "serialize"
    elif variant == 250:
        return "serializer"
    elif variant == 251:
        return "stringSlice"
    elif variant == 252:
        return "string"
    elif variant == 253:
        return "tensorFormat"
    elif variant == 254:
        return "tensorViewFormat"
    elif variant == 255:
        return "tensorDense"
    elif variant == 256:
        return "tensorStrided"
    elif variant == 257:
        return "tensorShape"
    elif variant == 258:
        return "tensorPlacement"
    elif variant == 259:
        return "tensorShardingAxis"
    elif variant == 260:
        return "tensorUnsharded"
    elif variant == 261:
        return "tensorShardingAxes"
    elif variant == 262:
        return "tensorShard"
    elif variant == 263:
        return "tensorReplicate"
    elif variant == 264:
        return "tensorPartial"
    elif variant == 265:
        return "tensor"
    elif variant == 266:
        return "tensorView"
    elif variant == 267:
        return "timeBinding"
    elif variant == 268:
        return "topologyBinding"
    elif variant == 269:
        return "tlsBinding"
    elif variant == 270:
        return "ttyBinding"
    elif variant == 271:
        return "constructorParameters"
    elif variant == 272:
        return "function"
    elif variant == 273:
        return "functionPointer"
    elif variant == 274:
        return "instanceType"
    elif variant == 275:
        return "omitThisParameter"
    elif variant == 276:
        return "parameters"
    elif variant == 277:
        return "returnType"
    elif variant == 278:
        return "thisParameterType"
    elif variant == 279:
        return "awaited"
    elif variant == 280:
        return "exclude"
    elif variant == 281:
        return "extract"
    elif variant == 282:
        return "nonNullable"
    elif variant == 283:
        return "noInfer"
    elif variant == 284:
        return "omit"
    elif variant == 285:
        return "partial"
    elif variant == 286:
        return "pick"
    elif variant == 287:
        return "propertyKey"
    elif variant == 288:
        return "readonly"
    elif variant == 289:
        return "record"
    elif variant == 290:
        return "required"
    elif variant == 291:
        return "thisType"
    elif variant == 292:
        return "capitalize"
    elif variant == 293:
        return "lowercase"
    elif variant == 294:
        return "uncapitalize"
    elif variant == 295:
        return "uppercase"
    elif variant == 296:
        return "option"
    elif variant == 297:
        return "symbol"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_language_item(value: LanguageItem) -> Json:
    """Return one JSON value for one LanguageItem."""
    return value


def from_json_language_item(value: Json) -> LanguageItem:
    """Return one LanguageItem from one JSON value."""
    variant = json_string(value)

    if variant == "accessibilityBinding":
        return "accessibilityBinding"
    elif variant == "asyncGenerator":
        return "asyncGenerator"
    elif variant == "generator":
        return "generator"
    elif variant == "generatorResult":
        return "generatorResult"
    elif variant == "generatorReturn":
        return "generatorReturn"
    elif variant == "generatorState":
        return "generatorState"
    elif variant == "generatorYield":
        return "generatorYield"
    elif variant == "asyncIterable":
        return "asyncIterable"
    elif variant == "asyncIterator":
        return "asyncIterator"
    elif variant == "continuationHandle":
        return "continuationHandle"
    elif variant == "continuationResult":
        return "continuationResult"
    elif variant == "continuationReturn":
        return "continuationReturn"
    elif variant == "continuationYield":
        return "continuationYield"
    elif variant == "queueMicrotask":
        return "queueMicrotask"
    elif variant == "suspendContinuation":
        return "suspendContinuation"
    elif variant == "promise":
        return "promise"
    elif variant == "promiseResolvers":
        return "promiseResolvers"
    elif variant == "audioBinding":
        return "audioBinding"
    elif variant == "array":
        return "array"
    elif variant == "fixedArray":
        return "fixedArray"
    elif variant == "readonlyArray":
        return "readonlyArray"
    elif variant == "map":
        return "map"
    elif variant == "set":
        return "set"
    elif variant == "slice":
        return "slice"
    elif variant == "computeBuffer":
        return "computeBuffer"
    elif variant == "computeDevice":
        return "computeDevice"
    elif variant == "computeMesh":
        return "computeMesh"
    elif variant == "computeKernel":
        return "computeKernel"
    elif variant == "computeKernelArgument":
        return "computeKernelArgument"
    elif variant == "computeProgram":
        return "computeProgram"
    elif variant == "computeEvent":
        return "computeEvent"
    elif variant == "computeStream":
        return "computeStream"
    elif variant == "context":
        return "context"
    elif variant == "currentContextValue":
        return "currentContextValue"
    elif variant == "contextKey":
        return "contextKey"
    elif variant == "contextPatch":
        return "contextPatch"
    elif variant == "contextToken":
        return "contextToken"
    elif variant == "contextEntry":
        return "contextEntry"
    elif variant == "currentContext":
        return "currentContext"
    elif variant == "getContextValue":
        return "getContextValue"
    elif variant == "popContext":
        return "popContext"
    elif variant == "pushContext":
        return "pushContext"
    elif variant == "requireContextValue":
        return "requireContextValue"
    elif variant == "as":
        return "as"
    elif variant == "borrow":
        return "borrow"
    elif variant == "toOwned":
        return "toOwned"
    elif variant == "from":
        return "from"
    elif variant == "tryFrom":
        return "tryFrom"
    elif variant == "into":
        return "into"
    elif variant == "tryInto":
        return "tryInto"
    elif variant == "cryptoBinding":
        return "cryptoBinding"
    elif variant == "capture":
        return "capture"
    elif variant == "cloneDerive":
        return "cloneDerive"
    elif variant == "debugDerive":
        return "debugDerive"
    elif variant == "tagged":
        return "tagged"
    elif variant == "allow":
        return "allow"
    elif variant == "deny":
        return "deny"
    elif variant == "expect":
        return "expect"
    elif variant == "forbid":
        return "forbid"
    elif variant == "warn":
        return "warn"
    elif variant == "extern":
        return "extern"
    elif variant == "intrinsic":
        return "intrinsic"
    elif variant == "languageItem":
        return "languageItem"
    elif variant == "reprDecorator":
        return "reprDecorator"
    elif variant == "noAliasingMutableBorrows":
        return "noAliasingMutableBorrows"
    elif variant == "noDynamicDispatch":
        return "noDynamicDispatch"
    elif variant == "noHeap":
        return "noHeap"
    elif variant == "noImplicitReceivers":
        return "noImplicitReceivers"
    elif variant == "noManaged":
        return "noManaged"
    elif variant == "noReflection":
        return "noReflection"
    elif variant == "noRuntime":
        return "noRuntime"
    elif variant == "noUnsafe":
        return "noUnsafe"
    elif variant == "noUnwind":
        return "noUnwind"
    elif variant == "deprecated":
        return "deprecated"
    elif variant == "experimental":
        return "experimental"
    elif variant == "cold":
        return "cold"
    elif variant == "hot":
        return "hot"
    elif variant == "inline":
        return "inline"
    elif variant == "likely":
        return "likely"
    elif variant == "mustUse":
        return "mustUse"
    elif variant == "noinline":
        return "noinline"
    elif variant == "pure":
        return "pure"
    elif variant == "tailcall":
        return "tailcall"
    elif variant == "unlikely":
        return "unlikely"
    elif variant == "unroll":
        return "unroll"
    elif variant == "safe":
        return "safe"
    elif variant == "sink":
        return "sink"
    elif variant == "source":
        return "source"
    elif variant == "taint":
        return "taint"
    elif variant == "unsafe":
        return "unsafe"
    elif variant == "untaint":
        return "untaint"
    elif variant == "deviceBinding":
        return "deviceBinding"
    elif variant == "displayBinding":
        return "displayBinding"
    elif variant == "error":
        return "error"
    elif variant == "abort":
        return "abort"
    elif variant == "panic":
        return "panic"
    elif variant == "panicValue":
        return "panicValue"
    elif variant == "setPanicHook":
        return "setPanicHook"
    elif variant == "takePanicHook":
        return "takePanicHook"
    elif variant == "todo":
        return "todo"
    elif variant == "unreachable":
        return "unreachable"
    elif variant == "asyncResult":
        return "asyncResult"
    elif variant == "err":
        return "err"
    elif variant == "ok":
        return "ok"
    elif variant == "result":
        return "result"
    elif variant == "fsBinding":
        return "fsBinding"
    elif variant == "gpuBinding":
        return "gpuBinding"
    elif variant == "inputBinding":
        return "inputBinding"
    elif variant == "ioBinding":
        return "ioBinding"
    elif variant == "ipcBinding":
        return "ipcBinding"
    elif variant == "extend":
        return "extend"
    elif variant == "fromIterator":
        return "fromIterator"
    elif variant == "iterable":
        return "iterable"
    elif variant == "iterator":
        return "iterator"
    elif variant == "eval":
        return "eval"
    elif variant == "expansionContext":
        return "expansionContext"
    elif variant == "macro":
        return "macro"
    elif variant == "macroContext":
        return "macroContext"
    elif variant == "materializationContext":
        return "materializationContext"
    elif variant == "bigInt":
        return "bigInt"
    elif variant == "complex":
        return "complex"
    elif variant == "math":
        return "math"
    elif variant == "number":
        return "number"
    elif variant == "vector":
        return "vector"
    elif variant == "access":
        return "access"
    elif variant == "dynamic":
        return "dynamic"
    elif variant == "allocationError":
        return "allocationError"
    elif variant == "memoryBinding":
        return "memoryBinding"
    elif variant == "arc":
        return "arc"
    elif variant == "arcInner":
        return "arcInner"
    elif variant == "arcWeak":
        return "arcWeak"
    elif variant == "borrowed":
        return "borrowed"
    elif variant == "box":
        return "box"
    elif variant == "clone":
        return "clone"
    elif variant == "concrete":
        return "concrete"
    elif variant == "copy":
        return "copy"
    elif variant == "default":
        return "default"
    elif variant == "dynamicSafe":
        return "dynamicSafe"
    elif variant == "send":
        return "send"
    elif variant == "sync":
        return "sync"
    elif variant == "unpin":
        return "unpin"
    elif variant == "zeroable":
        return "zeroable"
    elif variant == "unsafeCell":
        return "unsafeCell"
    elif variant == "asyncDispose":
        return "asyncDispose"
    elif variant == "dispose":
        return "dispose"
    elif variant == "drop":
        return "drop"
    elif variant == "forget":
        return "forget"
    elif variant == "manuallyDrop":
        return "manuallyDrop"
    elif variant == "maybeUninit":
        return "maybeUninit"
    elif variant == "lifetime":
        return "lifetime"
    elif variant == "managed":
        return "managed"
    elif variant == "owned":
        return "owned"
    elif variant == "phantom":
        return "phantom"
    elif variant == "pin":
        return "pin"
    elif variant == "place":
        return "place"
    elif variant == "placed":
        return "placed"
    elif variant == "space":
        return "space"
    elif variant == "raw":
        return "raw"
    elif variant == "rc":
        return "rc"
    elif variant == "rcInner":
        return "rcInner"
    elif variant == "rcWeak":
        return "rcWeak"
    elif variant == "accessOf":
        return "accessOf"
    elif variant == "accessOr":
        return "accessOr"
    elif variant == "baseOf":
        return "baseOf"
    elif variant == "isBorrowed":
        return "isBorrowed"
    elif variant == "isManaged":
        return "isManaged"
    elif variant == "isOwned":
        return "isOwned"
    elif variant == "isRaw":
        return "isRaw"
    elif variant == "isShared":
        return "isShared"
    elif variant == "isSharedIn":
        return "isSharedIn"
    elif variant == "lifetimeOf":
        return "lifetimeOf"
    elif variant == "lifetimeOr":
        return "lifetimeOr"
    elif variant == "ownership":
        return "ownership"
    elif variant == "ownershipOf":
        return "ownershipOf"
    elif variant == "ownershipOr":
        return "ownershipOr"
    elif variant == "payloadOf":
        return "payloadOf"
    elif variant == "placeIn":
        return "placeIn"
    elif variant == "placeOf":
        return "placeOf"
    elif variant == "placeOr":
        return "placeOr"
    elif variant == "spaceOf":
        return "spaceOf"
    elif variant == "spaceOr":
        return "spaceOr"
    elif variant == "withAccess":
        return "withAccess"
    elif variant == "withBase":
        return "withBase"
    elif variant == "withLifetime":
        return "withLifetime"
    elif variant == "withOwnership":
        return "withOwnership"
    elif variant == "withPlace":
        return "withPlace"
    elif variant == "withSpace":
        return "withSpace"
    elif variant == "unique":
        return "unique"
    elif variant == "importMeta":
        return "importMeta"
    elif variant == "importMetaEnv":
        return "importMetaEnv"
    elif variant == "netBinding":
        return "netBinding"
    elif variant == "and":
        return "and"
    elif variant == "not":
        return "not"
    elif variant == "or":
        return "or"
    elif variant == "xor":
        return "xor"
    elif variant == "compare":
        return "compare"
    elif variant == "ordering":
        return "ordering"
    elif variant == "partialCompare":
        return "partialCompare"
    elif variant == "dereference":
        return "dereference"
    elif variant == "divide":
        return "divide"
    elif variant == "equal":
        return "equal"
    elif variant == "partialEqual":
        return "partialEqual"
    elif variant == "debug":
        return "debug"
    elif variant == "display":
        return "display"
    elif variant == "hash":
        return "hash"
    elif variant == "hasher":
        return "hasher"
    elif variant == "subtract":
        return "subtract"
    elif variant == "multiply":
        return "multiply"
    elif variant == "negate":
        return "negate"
    elif variant == "add":
        return "add"
    elif variant == "plus":
        return "plus"
    elif variant == "power":
        return "power"
    elif variant == "remainder":
        return "remainder"
    elif variant == "shiftLeft":
        return "shiftLeft"
    elif variant == "shiftRight":
        return "shiftRight"
    elif variant == "shiftRightUnsigned":
        return "shiftRightUnsigned"
    elif variant == "index":
        return "index"
    elif variant == "indexSet":
        return "indexSet"
    elif variant == "fromResidual":
        return "fromResidual"
    elif variant == "try":
        return "try"
    elif variant == "controlFlow":
        return "controlFlow"
    elif variant == "osBinding":
        return "osBinding"
    elif variant == "processBinding":
        return "processBinding"
    elif variant == "randomBinding":
        return "randomBinding"
    elif variant == "bound":
        return "bound"
    elif variant == "rangeBounds":
        return "rangeBounds"
    elif variant == "range":
        return "range"
    elif variant == "rangeFrom":
        return "rangeFrom"
    elif variant == "rangeFull":
        return "rangeFull"
    elif variant == "rangeInclusive":
        return "rangeInclusive"
    elif variant == "rangeTo":
        return "rangeTo"
    elif variant == "rangeToInclusive":
        return "rangeToInclusive"
    elif variant == "step":
        return "step"
    elif variant == "reflect":
        return "reflect"
    elif variant == "alignOf":
        return "alignOf"
    elif variant == "layout":
        return "layout"
    elif variant == "layoutField":
        return "layoutField"
    elif variant == "layoutOf":
        return "layoutOf"
    elif variant == "layoutShape":
        return "layoutShape"
    elif variant == "layoutVariant":
        return "layoutVariant"
    elif variant == "sizeOf":
        return "sizeOf"
    elif variant == "strideOf":
        return "strideOf"
    elif variant == "type":
        return "type"
    elif variant == "typeId":
        return "typeId"
    elif variant == "typeOf":
        return "typeOf"
    elif variant == "regExp":
        return "regExp"
    elif variant == "binding":
        return "binding"
    elif variant == "deserialize":
        return "deserialize"
    elif variant == "deserializer":
        return "deserializer"
    elif variant == "serialize":
        return "serialize"
    elif variant == "serializer":
        return "serializer"
    elif variant == "stringSlice":
        return "stringSlice"
    elif variant == "string":
        return "string"
    elif variant == "tensorFormat":
        return "tensorFormat"
    elif variant == "tensorViewFormat":
        return "tensorViewFormat"
    elif variant == "tensorDense":
        return "tensorDense"
    elif variant == "tensorStrided":
        return "tensorStrided"
    elif variant == "tensorShape":
        return "tensorShape"
    elif variant == "tensorPlacement":
        return "tensorPlacement"
    elif variant == "tensorShardingAxis":
        return "tensorShardingAxis"
    elif variant == "tensorUnsharded":
        return "tensorUnsharded"
    elif variant == "tensorShardingAxes":
        return "tensorShardingAxes"
    elif variant == "tensorShard":
        return "tensorShard"
    elif variant == "tensorReplicate":
        return "tensorReplicate"
    elif variant == "tensorPartial":
        return "tensorPartial"
    elif variant == "tensor":
        return "tensor"
    elif variant == "tensorView":
        return "tensorView"
    elif variant == "timeBinding":
        return "timeBinding"
    elif variant == "topologyBinding":
        return "topologyBinding"
    elif variant == "tlsBinding":
        return "tlsBinding"
    elif variant == "ttyBinding":
        return "ttyBinding"
    elif variant == "constructorParameters":
        return "constructorParameters"
    elif variant == "function":
        return "function"
    elif variant == "functionPointer":
        return "functionPointer"
    elif variant == "instanceType":
        return "instanceType"
    elif variant == "omitThisParameter":
        return "omitThisParameter"
    elif variant == "parameters":
        return "parameters"
    elif variant == "returnType":
        return "returnType"
    elif variant == "thisParameterType":
        return "thisParameterType"
    elif variant == "awaited":
        return "awaited"
    elif variant == "exclude":
        return "exclude"
    elif variant == "extract":
        return "extract"
    elif variant == "nonNullable":
        return "nonNullable"
    elif variant == "noInfer":
        return "noInfer"
    elif variant == "omit":
        return "omit"
    elif variant == "partial":
        return "partial"
    elif variant == "pick":
        return "pick"
    elif variant == "propertyKey":
        return "propertyKey"
    elif variant == "readonly":
        return "readonly"
    elif variant == "record":
        return "record"
    elif variant == "required":
        return "required"
    elif variant == "thisType":
        return "thisType"
    elif variant == "capitalize":
        return "capitalize"
    elif variant == "lowercase":
        return "lowercase"
    elif variant == "uncapitalize":
        return "uncapitalize"
    elif variant == "uppercase":
        return "uppercase"
    elif variant == "option":
        return "option"
    elif variant == "symbol":
        return "symbol"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "LanguageItem",
    "encode_language_item",
    "decode_language_item",
    "to_json_language_item",
    "from_json_language_item",
]
