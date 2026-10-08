/** Stand in for a DOM node the server must never touch, failing on any use. */
export function untouched<Target extends object>(): Target;
/**
 * Make the stand-in.
 *
 * @construct a proxy that throws on every trap stands in for any object type
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
