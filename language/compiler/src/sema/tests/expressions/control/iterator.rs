use crate::tests::{DirRows, TestSession};

#[test]
fn test_for_of_binds_array_elements() {
    let session = TestSession::single(
        r#"
const values: int32[] = [1, 2, 3];

for (const value of values) {
    value satisfies int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: int32[] = [1, 2, 3];

for (const value of values) {
    value satisfies int32;
}

=== dir ===
const values: int32[] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @type.node source=[1, 2, 3] type=int32[]
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest(1, 2, 3) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
/// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
/// @generic.instance id=size<int32> template=size arguments=(int32)
/// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
/// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

for (const value of values) {
/// @resolution.iteration iterator="iterator#2(parameters=(), arguments=(), return=Iterator<int32>)" next="dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>)"
/// @generic.instantiation id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
/// @generic.instance id="IteratorResult<int32, void>" template=IteratorResult arguments=(int32, void)
/// @generic.instance id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
/// @generic.instance id=Iterator<int32> template=Iterator arguments=(int32)
/// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
/// @generic.instance id=IteratorYield<int32> template=IteratorYield arguments=(int32)
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=values type=int32[]
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=values root=values

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=value root=value

}
"#,
    );
}

#[test]
fn test_for_of_rejects_non_iterable_receiver() {
    let session = TestSession::single(
        r#"
for (const value of 1) {
    value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
for (const value of 1) {
    value;
}

=== dir ===
for (const value of 1) {
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

    value;
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=value root=value

}
"#,
        r#"
/// @diagnostic.error id=for-of-source-not-iterable message="for-of source must be iterable"
/// @diagnostic.label line=2 column=21 span="1" line_source="for (const value of 1) {"
"#,
    );
}

#[test]
fn test_for_of_binding_accepts_compound_assignment() {
    let session = TestSession::single(
        r#"
function visit(values: int32[]): void {
    for (let value of values) {
        value += 1;
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function visit(values: int32[]): void {
    for (let value of values) {
        value += 1;
    }
}

=== dir ===
function visit(values: int32[]): void {
/// @type.symbol symbol=visit type=(int32[]) => void
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @type.symbol symbol=visit.values source="values: int32[]" type=int32[]

    for (let value of values) {
    /// @resolution.iteration iterator="iterator#2(parameters=(), arguments=(), return=Iterator<int32>)" next="dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>)"
    /// @generic.instantiation id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
    /// @generic.instance id="IteratorResult<int32, void>" template=IteratorResult arguments=(int32, void)
    /// @generic.instance id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
    /// @generic.instance id=Iterator<int32> template=Iterator arguments=(int32)
    /// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
    /// @generic.instance id=IteratorYield<int32> template=IteratorYield arguments=(int32)
    /// @type.symbol symbol=visit.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=visit.value
    /// @resolution.name source=values target=visit.values
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=visit.values

        value += 1;
        /// @resolution.name source=value target=visit.value
        /// @resolution.operator source="value += 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.pattern.assign source=value kind=place
        /// @resolution.place source=value placement="local" lifetime="frame" access="mutable"
        /// @resolution.assignment source=value read=binding(visit.value) write=binding(visit.value) type=int32
        /// @resolution.access source=value root=visit.value

    }
}
"#,
    );
}

#[test]
fn test_for_of_accepts_an_iterator_through_the_blanket_iterable() {
    let session = TestSession::single(
        r#"
function total(values: int32[]): int32 {
    let sum: int32 = 0;
    for (const (index, value) of values.iterator().enumerate()) {
        sum = sum + value;
    }
    return sum;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function total(values: int32[]): int32 {
    let sum: int32 = 0;
    for (const (index, value) of values.iterator<int32>().enumerate<int32>()) {
        sum = sum + value;
    }
    return sum;
}

=== dir ===
function total(values: int32[]): int32 {
/// @type.symbol symbol=total type=(int32[]) => int32
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @type.symbol symbol=total.values source="values: int32[]" type=int32[]

    let sum: int32 = 0;
    /// @type.symbol symbol=total.sum source=sum type=int32
    /// @resolution.pattern source=sum kind=binding target=total.sum

    for (const (index, value) of values.iterator().enumerate()) {
    /// @resolution.iteration iterator="iterator(parameters=(), arguments=(), return=EnumeratedIterator<Iterator<int32>, int32>)" next="next#1(parameters=(), arguments=(), return=IteratorResult<(isize, int32), void>, regions=(\"frame\" & \"local\"))"
    /// @generic.instantiation id="iterator<(isize, int32), EnumeratedIterator<Iterator<int32>, int32>>" template=iterator arguments=((isize, int32), EnumeratedIterator<Iterator<int32>, int32>)
    /// @generic.instantiation id="next#1<int32, Iterator<int32>>" template=next#1 arguments=(int32, Iterator<int32>)
    /// @generic.instance id="IteratorResult<(isize, int32), void>" template=IteratorResult arguments=((isize, int32), void)
    /// @generic.instance id="IteratorYield<(isize, int32)>" template=IteratorYield arguments=((isize, int32))
    /// @generic.instance id="iterator<(isize, int32), EnumeratedIterator<Iterator<int32>, int32>>" template=iterator arguments=((isize, int32), EnumeratedIterator<Iterator<int32>, int32>)
    /// @generic.instance id="next#1<int32, Iterator<int32>>" template=next#1 arguments=(int32, Iterator<int32>)
    /// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
    /// @resolution.pattern source=(index, value) kind=tuple fields=(total.index, total.value)
    /// @type.symbol symbol=total.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=total.index
    /// @type.symbol symbol=total.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=total.value
    /// @resolution.name source=values target=total.values
    /// @resolution.member source=values.iterator receiver=int32[] type=<iterator#2.P0: Place>(this: Managed<int32[], iterator#2.P0>) => Iterator<int32> kind=symbol target_receiver=int32[] target=iterator#2
    /// @resolution.member source=values.iterator().enumerate receiver=Iterator<int32> type=(this: Iterator<int32>) => EnumeratedIterator<Iterator<int32>, int32> kind=symbol target_receiver=Iterator<int32> dispatch=dynamic constraint=Iterator<int32> target=Iterator.enumerate
    /// @resolution.call source=values.iterator() parameters=() return=Iterator<int32> kind=symbol target=iterator#2 receiver=int32[] instance="Array<int32>.<extension#4>.iterator#2<\"local\">"
    /// @resolution.call source=values.iterator().enumerate() parameters=() return=EnumeratedIterator<Iterator<int32>, int32> kind=dynamic target=Iterator.enumerate receiver=Iterator<int32> constraint=Iterator<int32> generic_arguments=(int32)
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=total.values
    /// @generic.instantiation id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
    /// @generic.instantiation id=Iterator.enumerate<int32> template=Iterator.enumerate arguments=(int32)
    /// @generic.instantiation id=iterator#2<int32> template=iterator#2 arguments=(int32)
    /// @generic.instance id="EnumeratedIterator<Iterator<int32>, int32>" template=EnumeratedIterator arguments=(Iterator<int32>, int32)
    /// @generic.instance id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
    /// @generic.instance id=Iterator<int32> template=Iterator arguments=(int32)

        sum = sum + value;
        /// @resolution.name source=sum target=total.sum
        /// @resolution.pattern.assign source=sum kind=place
        /// @resolution.access source=sum root=total.sum
        /// @resolution.assignment source=sum write=binding(total.sum) type=int32
        /// @resolution.name source=sum target=total.sum
        /// @resolution.operator source="sum + value" type=int32 operator="+" kind=builtin operands=[sum as int32 families=(integer), value as int32 families=(integer)]
        /// @resolution.place source=sum placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=sum root=total.sum
        /// @resolution.name source=value target=total.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=value root=total.value

    }
    return sum;
    /// @resolution.name source=sum target=total.sum
    /// @resolution.place source=sum placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=sum root=total.sum

}
"#,
    );
}
