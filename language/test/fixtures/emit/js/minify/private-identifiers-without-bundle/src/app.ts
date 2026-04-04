class Foo {
    #foo;
    foo = class {
        #foo;
        #foo2;
        #bar;
    };
    get #bar() {}
    set #bar(value: unknown) {}
}

class Bar {
    #foo;
    foo = class {
        #foo2;
        #foo;
        #bar;
    };
    get #bar() {}
    set #bar(value: unknown) {}
}

export const privateIdentifierShapes = [Foo, Bar];
