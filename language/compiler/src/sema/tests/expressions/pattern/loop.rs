use crate::tests::{DirRows, TestSession};

#[test]
fn test_for_of_destructures_object_elements() {
    let session = TestSession::single(
        r#"
declare const items: { name: string; value: int32 }[];

for (const { name, value } of items) {
    name satisfies string;
    value satisfies int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const items: { name: string; value: int32 }[];

for (const { name, value } of items) {
    name satisfies string;
    value satisfies int32;
}

=== dir ===
declare const items: { name: string; value: int32 }[];
/// @type.symbol symbol=items source=items type={ name: string; value: int32 }[]
/// @resolution.pattern source=items kind=binding target=items
/// @generic.instance id="Array<{ name: string; value: int32 }>" template=Array arguments=({ name: string; value: int32 })
/// @generic.instance id="MaybeUninit<{ name: string; value: int32 }>" template=MaybeUninit arguments=({ name: string; value: int32 })
/// @generic.instance id="new<MaybeUninit<{ name: string; value: int32 }>>" template=new arguments=(MaybeUninit<{ name: string; value: int32 }>)
/// @type.symbol symbol=name#1 source="name: string" type=string
/// @type.symbol symbol=value#1 source="value: int32" type=int32

for (const { name, value } of items) {
/// @resolution.iteration iterator="iterator#2(parameters=(), arguments=(), return=Iterator<{ name: string; value: int32 }>)" next="dynamic(Iterator<{ name: string; value: int32 }> as Iterator<{ name: string; value: int32 }>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<{ name: string; value: int32 }, void>)"
/// @generic.instantiation id="iterator#2<{ name: string; value: int32 }, \"local\">" template=iterator#2 arguments=({ name: string; value: int32 }, "local")
/// @generic.instance id="DropIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }>" template=DropIterator arguments=(Iterator<{ name: string; value: int32 }>, { name: string; value: int32 })
/// @generic.instance id="DropWhileIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }>" template=DropWhileIterator arguments=(Iterator<{ name: string; value: int32 }>, { name: string; value: int32 })
/// @generic.instance id="EnumeratedIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }>" template=EnumeratedIterator arguments=(Iterator<{ name: string; value: int32 }>, { name: string; value: int32 })
/// @generic.instance id="FilterIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }>" template=FilterIterator arguments=(Iterator<{ name: string; value: int32 }>, { name: string; value: int32 })
/// @generic.instance id="InspectIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }>" template=InspectIterator arguments=(Iterator<{ name: string; value: int32 }>, { name: string; value: int32 })
/// @generic.instance id="Iterator.collect<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }, ^{ name: string; value: int32 }[]>" template=Iterator.collect arguments=({ name: string; value: int32 }, ^{ name: string; value: int32 }[])
/// @generic.instance id="Iterator<{ name: string; value: int32 }>" template=Iterator arguments=({ name: string; value: int32 }) evaluated=(<Iterator.flatMap.U, Iterator.flatMap.V: Iterable<Iterator.flatMap.U>>(this: this, Function<(Iterator.T, isize), Iterator.flatMap.V>) => FlatMapIterator<this, Iterator.T, Iterator.flatMap.V, Iterator.flatMap.V.Iterator, Iterator.flatMap.U> => <Iterator.flatMap.U, Iterator.flatMap.V: Iterable<Iterator.flatMap.U>>(this: Iterator<{ name: string; value: int32 }>, Function<({ name: string; value: int32 }, isize), Iterator.flatMap.V>) => FlatMapIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }, Iterator.flatMap.V, Iterator.flatMap.V.Iterator, Iterator.flatMap.U>, <Iterator.chain.V: Iterable<Iterator.T>>(this: this, Iterator.chain.V) => ChainIterator<this, Iterator.chain.V.Iterator, Iterator.T> => <Iterator.chain.V: Iterable<Iterator.T>>(this: Iterator<{ name: string; value: int32 }>, Iterator.chain.V) => ChainIterator<Iterator<{ name: string; value: int32 }>, Iterator.chain.V.Iterator, { name: string; value: int32 }>, <Iterator.zip.U, Iterator.zip.V: Iterable<Iterator.zip.U>>(this: this, Iterator.zip.V) => ZipIterator<this, Iterator.zip.V.Iterator, Iterator.T, Iterator.zip.U> => <Iterator.zip.U, Iterator.zip.V: Iterable<Iterator.zip.U>>(this: Iterator<{ name: string; value: int32 }>, Iterator.zip.V) => ZipIterator<Iterator<{ name: string; value: int32 }>, Iterator.zip.V.Iterator, { name: string; value: int32 }, Iterator.zip.U>, <Iterator.tryFold.F: Try>(this: this, Iterator.tryFold.F.Output, Function<(Iterator.tryFold.F.Output, Iterator.T, isize), Iterator.tryFold.F>) => Iterator.tryFold.F => <Iterator.tryFold.F: Try>(this: Iterator<{ name: string; value: int32 }>, Iterator.tryFold.F.Output, Function<(Iterator.tryFold.F.Output, { name: string; value: int32 }, isize), Iterator.tryFold.F>) => Iterator.tryFold.F)
/// @generic.instance id="IteratorResult<{ name: string; value: int32 }, void>" template=IteratorResult arguments=({ name: string; value: int32 }, void)
/// @generic.instance id="IteratorYield<{ name: string; value: int32 }>" template=IteratorYield arguments=({ name: string; value: int32 })
/// @generic.instance id="PeekableIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }>" template=PeekableIterator arguments=(Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }) evaluated=(IteratorResult<PeekableIterator.T, PeekableIterator.I.Return> | undefined => IteratorResult<{ name: string; value: int32 }, void> | undefined)
/// @generic.instance id="TakeIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }>" template=TakeIterator arguments=(Iterator<{ name: string; value: int32 }>, { name: string; value: int32 })
/// @generic.instance id="TakeWhileIterator<Iterator<{ name: string; value: int32 }>, { name: string; value: int32 }>" template=TakeWhileIterator arguments=(Iterator<{ name: string; value: int32 }>, { name: string; value: int32 })
/// @generic.instance id="iterator#2<{ name: string; value: int32 }, \"local\">" template=iterator#2 arguments=({ name: string; value: int32 }, "local")
/// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
/// @resolution.pattern source={ name, value } kind=object fields={ name, value }
/// @type.symbol symbol=name#2 source=name type=string
/// @type.symbol symbol=value#2 source=value type=int32
/// @type.node source=items type={ name: string; value: int32 }[]
/// @resolution.name source=items target=items
/// @resolution.place source=items placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=items root=items

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=name#2

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=value root=value#2

}
"#,
    );
}
