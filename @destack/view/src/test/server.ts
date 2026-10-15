/** Stand in for a DOM node the server must never touch, failing on any use. */
export function untouched<Target extends object>(): Target;
/**
 * Make the stand-in.
 *
 * NOTE #Architecture: a throwing proxy stands in for any node the server must not touch.
 */
export function untouched(): object {
    return new Proxy(
        {},
        {
            get: (_target, key) => {
                throw new TypeError(`the server read ${String(key)} of a DOM node`);
            },
            set: (_target, key) => {
                throw new TypeError(`the server wrote ${String(key)} of a DOM node`);
            },
        },
    );
}
