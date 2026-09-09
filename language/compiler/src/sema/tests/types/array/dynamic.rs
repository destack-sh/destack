use crate::tests::{DirRows, TestSession};

#[test]
fn test_dynamic_array_subscript_selects_element_type() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
const byte = bytes[index];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
const byte: uint8 = bytes[index];

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=uint8[]
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id="initAsPointer<uint8, \"mutable\">" template=initAsPointer arguments=(uint8, "mutable")
/// @generic.instance id=Array<uint8> template=Array arguments=(uint8)
/// @generic.instance id=assumeInitDrop#1<uint8> template=assumeInitDrop#1 arguments=(uint8)
/// @generic.instance id=assumeInitDrop<uint8> template=assumeInitDrop arguments=(uint8)
/// @generic.instance id=clear<uint8> template=clear arguments=(uint8)
/// @generic.instance id=drop<uint8> template=drop arguments=(uint8)
/// @generic.instance id=dropInPlace<uint8> template=dropInPlace arguments=(uint8)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<uint8>> template=sliceAssumeInit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceUninit<MaybeUninit<uint8>> template=sliceUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=truncate<uint8> template=truncate arguments=(uint8)

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

const byte = bytes[index];
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.pattern source=byte kind=binding target=byte
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=bytes root=bytes
/// @resolution.subscript source=bytes[index] type=uint8 kind=call target="index#1(parameters=(isize), arguments=(provided(index) as isize), return=WithAccess<Borrowed<uint8, \"managed\" & \"local\", \"mutable\">, \"mutable\">, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#1<uint8, \"mutable\">" template=index#1 arguments=(uint8, "mutable")
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="WithAccess<&'bound0 uint8, \"mutable\">" template=WithAccess arguments=(&'bound0 uint8, "mutable")
/// @generic.instance id="WithAccess<&'bound0 uint8[], \"mutable\">" template=WithAccess arguments=(&'bound0 uint8[], "mutable")
/// @generic.instance id="assumeInitReference<uint8, \"mutable\">" template=assumeInitReference arguments=(uint8, "mutable")
/// @generic.instance id="elementSlot<uint8, \"mutable\">" template=elementSlot arguments=(uint8, "mutable")
/// @generic.instance id="index#1<uint8, \"mutable\">" template=index#1 arguments=(uint8, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<uint8>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<uint8>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=elementPosition<uint8> template=elementPosition arguments=(uint8)
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}

#[test]
fn test_dynamic_array_subscript_write_selects_index_set() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
bytes[index] = 255;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
bytes[index] = 255;

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=uint8[]
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="initAsPointer<uint8, \"mutable\">" template=initAsPointer arguments=(uint8, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<uint8> template=Array arguments=(uint8)
/// @generic.instance id=assumeInitDrop#1<uint8> template=assumeInitDrop#1 arguments=(uint8)
/// @generic.instance id=assumeInitDrop<uint8> template=assumeInitDrop arguments=(uint8)
/// @generic.instance id=clear<uint8> template=clear arguments=(uint8)
/// @generic.instance id=drop<uint8> template=drop arguments=(uint8)
/// @generic.instance id=dropInPlace<uint8> template=dropInPlace arguments=(uint8)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<uint8>> template=sliceAssumeInit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceUninit<MaybeUninit<uint8>> template=sliceUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=truncate<uint8> template=truncate arguments=(uint8)

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

bytes[index] = 255;
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=bytes root=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place
/// @resolution.assignment source=bytes[index] write="indexSet#1(parameters=(isize, uint8), arguments=(provided(index) as isize, supplied as uint8), return=void, regions=(\"managed\" & \"local\"))" type=uint8
/// @generic.instantiation id=indexSet#1<uint8> template=indexSet#1 arguments=(uint8)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="as<uint8, \"mutable\" | \"readonly\">" template=as arguments=(uint8, "mutable" | "readonly")
/// @generic.instance id="assumeInitReference<uint8, \"mutable\">" template=assumeInitReference arguments=(uint8, "mutable")
/// @generic.instance id="elementSlot<uint8, \"mutable\">" template=elementSlot arguments=(uint8, "mutable")
/// @generic.instance id="index#1<uint8, \"mutable\">" template=index#1 arguments=(uint8, "mutable")
/// @generic.instance id="index#2<uint8, RangeBounds<isize>, \"mutable\" | \"readonly\">" template=index#2 arguments=(uint8, RangeBounds<isize>, "mutable" | "readonly")
/// @generic.instance id="rangeSpan<uint8, RangeBounds<isize>, \"mutable\" | \"readonly\">" template=rangeSpan arguments=(uint8, RangeBounds<isize>, "mutable" | "readonly")
/// @generic.instance id="sliceIndex<MaybeUninit<uint8>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<uint8>, "mutable")
/// @generic.instance id="sliceView<uint8, \"mutable\" | \"readonly\">" template=sliceView arguments=(uint8, "mutable" | "readonly")
/// @generic.instance id="subslice<uint8, \"mutable\" | \"readonly\">" template=subslice arguments=(uint8, "mutable" | "readonly")
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=elementPosition<uint8> template=elementPosition arguments=(uint8)
/// @generic.instance id=indexSet#1<uint8> template=indexSet#1 arguments=(uint8)
/// @generic.instance id=size<uint8> template=size arguments=(uint8)
/// @generic.instance id=sliceLength<uint8> template=sliceLength arguments=(uint8)
/// @generic.instance id=symbol2<uint8> template=symbol2 arguments=(uint8)
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}

#[test]
fn test_dynamic_array_compound_subscript_resolves_read_and_write() {
    let session = TestSession::single(
        r#"
declare const bytes: uint8[];
declare const index: isize;
bytes[index] += 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const bytes: uint8[];
declare const index: isize;
bytes[index] += 1;

=== dir ===
declare const bytes: uint8[];
/// @type.symbol symbol=bytes source=bytes type=uint8[]
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @generic.instance id="initAsPointer<uint8, \"mutable\">" template=initAsPointer arguments=(uint8, "mutable")
/// @generic.instance id=Array<uint8> template=Array arguments=(uint8)
/// @generic.instance id=assumeInitDrop#1<uint8> template=assumeInitDrop#1 arguments=(uint8)
/// @generic.instance id=assumeInitDrop<uint8> template=assumeInitDrop arguments=(uint8)
/// @generic.instance id=clear<uint8> template=clear arguments=(uint8)
/// @generic.instance id=drop<uint8> template=drop arguments=(uint8)
/// @generic.instance id=dropInPlace<uint8> template=dropInPlace arguments=(uint8)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<uint8>> template=sliceAssumeInit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=sliceUninit<MaybeUninit<uint8>> template=sliceUninit arguments=(MaybeUninit<uint8>)
/// @generic.instance id=truncate<uint8> template=truncate arguments=(uint8)

declare const index: isize;
/// @type.symbol symbol=index source=index type=isize
/// @resolution.pattern source=index kind=binding target=index

bytes[index] += 1;
/// @resolution.name source=bytes target=bytes
/// @resolution.operator source="bytes[index] += 1" type=uint8 operator="+" kind=builtin operands=[bytes[index] as uint8 families=(integer), 1 as uint8 families=(integer)]
/// @resolution.place source=bytes placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=bytes root=bytes
/// @resolution.pattern.assign source=bytes[index] kind=place
/// @resolution.assignment source=bytes[index] read="index#1(parameters=(isize), arguments=(provided(index) as isize), return=WithAccess<Borrowed<uint8, \"managed\" & \"local\", \"mutable\">, \"mutable\">, regions=(\"managed\" & \"local\"))" write="indexSet#1(parameters=(isize, uint8), arguments=(provided(index) as isize, supplied as uint8), return=void, regions=(\"managed\" & \"local\"))" type=uint8
/// @generic.instantiation id="index#1<uint8, \"mutable\">" template=index#1 arguments=(uint8, "mutable")
/// @generic.instantiation id=indexSet#1<uint8> template=indexSet#1 arguments=(uint8)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="WithAccess<&'bound0 uint8, \"mutable\">" template=WithAccess arguments=(&'bound0 uint8, "mutable")
/// @generic.instance id="WithAccess<&'bound0 uint8[], \"mutable\">" template=WithAccess arguments=(&'bound0 uint8[], "mutable")
/// @generic.instance id="as<uint8, \"mutable\" | \"readonly\">" template=as arguments=(uint8, "mutable" | "readonly")
/// @generic.instance id="assumeInitReference<uint8, \"mutable\">" template=assumeInitReference arguments=(uint8, "mutable")
/// @generic.instance id="elementSlot<uint8, \"mutable\">" template=elementSlot arguments=(uint8, "mutable")
/// @generic.instance id="index#1<uint8, \"mutable\">" template=index#1 arguments=(uint8, "mutable")
/// @generic.instance id="index#2<uint8, RangeBounds<isize>, \"mutable\" | \"readonly\">" template=index#2 arguments=(uint8, RangeBounds<isize>, "mutable" | "readonly")
/// @generic.instance id="rangeSpan<uint8, RangeBounds<isize>, \"mutable\" | \"readonly\">" template=rangeSpan arguments=(uint8, RangeBounds<isize>, "mutable" | "readonly")
/// @generic.instance id="sliceIndex<MaybeUninit<uint8>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<uint8>, "mutable")
/// @generic.instance id="sliceView<uint8, \"mutable\" | \"readonly\">" template=sliceView arguments=(uint8, "mutable" | "readonly")
/// @generic.instance id="subslice<uint8, \"mutable\" | \"readonly\">" template=subslice arguments=(uint8, "mutable" | "readonly")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=elementPosition<uint8> template=elementPosition arguments=(uint8)
/// @generic.instance id=indexSet#1<uint8> template=indexSet#1 arguments=(uint8)
/// @generic.instance id=size<uint8> template=size arguments=(uint8)
/// @generic.instance id=sliceLength<uint8> template=sliceLength arguments=(uint8)
/// @generic.instance id=symbol2<uint8> template=symbol2 arguments=(uint8)
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=index root=index
"#,
    );
}

/// A scalar written into an `unknown` element erases behind the element's handle.
#[test]
fn test_write_a_scalar_into_an_unknown_array_element() {
    let session = TestSession::single(
        r#"
function copy(target: &unknown[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function copy<'a, 'b>(target: &'a unknown[], source: &'b readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index] as Managed<unknown, 'a>;
    }
}

=== dir ===
function copy(target: &unknown[], source: &readonly int32[]): void {
/// @generic.template symbol=copy parameters=('a, 'b)
/// @type.symbol symbol=copy type=<copy.'a, copy.'b>(&copy.'a unknown[], &copy.'b readonly int32[]) => void
/// @type.symbol symbol=copy.target source="target: &unknown[]" type=&copy.'a unknown[]
/// @type.symbol symbol=copy.source source="source: &readonly int32[]" type=&copy.'b readonly int32[]

    for (let index: isize = 0; index < source.length; index++) {
    /// @type.symbol symbol=copy.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=copy.index
    /// @resolution.name source=index target=copy.index
    /// @resolution.operator source="index < source.length" type=boolean operator="<" kind=builtin operands=[index as isize families=(integer), source.length as isize families=(integer)]
    /// @resolution.place source=index placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=index root=copy.index
    /// @resolution.name source=source target=copy.source
    /// @resolution.member source=source.length receiver=&copy.'b readonly int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(copy.'b))"
    /// @resolution.place source=source placement=copy.'b lifetime=copy.'b access="readonly"
    /// @resolution.access source=source root=copy.source
    /// @generic.instantiation id=length<int32> template=length arguments=(int32)
    /// @resolution.name source=index target=copy.index
    /// @resolution.assignment source=index read=binding(copy.index) write=binding(copy.index) type=isize
    /// @resolution.access source=index root=copy.index
    /// @resolution.operator source=index++ type=isize operator="++" kind=builtin operands=[index as isize families=(integer)]

        target[index] = source[index];
        /// @resolution.name source=target target=copy.target
        /// @resolution.place source=target placement=copy.'a lifetime=copy.'a access="mutable"
        /// @resolution.access source=target root=copy.target
        /// @resolution.pattern.assign source=target[index] kind=place
        /// @resolution.assignment source=target[index] write="indexSet#1(parameters=(isize, Managed<unknown, copy.'a>), arguments=(provided(index) as isize, supplied as Managed<unknown, copy.'a>), return=void, regions=(copy.'a))" type=Managed<unknown, copy.'a>
        /// @generic.instantiation id=indexSet#1<unknown> template=indexSet#1 arguments=(unknown)
        /// @resolution.name source=index target=copy.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=index root=copy.index
        /// @resolution.name source=source target=copy.source
        /// @resolution.place source=source placement=copy.'b lifetime=copy.'b access="readonly"
        /// @resolution.access source=source root=copy.source
        /// @resolution.place source=source[index] placement=copy.'b lifetime=copy.'b access="readonly"
        /// @resolution.subscript source=source[index] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(index) as isize), return=WithAccess<&copy.'b int32, \"readonly\">, regions=(copy.'b))"
        /// @generic.instantiation id="index#1<int32, \"readonly\">" template=index#1 arguments=(int32, "readonly")
        /// @resolution.name source=index target=copy.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=index root=copy.index

    }
}
"#,
        r#"
"#,
    );
}
