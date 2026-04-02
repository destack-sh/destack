class b{
    private #foo
    foo = class{
        private #foo
        private #foo2
        private #bar
    }
    private get #bar() {
    }
    private set #bar(c) {
    }
}
class a{
    private #foo
    foo = class{
        private #foo2
        private #foo
        private #bar
    }
    private get #bar() {
    }
    private set #bar(d) {
    }
}
export const privateIdentifierShapes = [b, a];
