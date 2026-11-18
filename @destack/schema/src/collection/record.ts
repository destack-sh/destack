/** Build a record from an object's own keys, typed as the mapped type its caller states. */
export function mapValues<Source extends object, Mapped extends object>(
    source: Source,
    map: (value: Source[keyof Source], key: keyof Source & string) => unknown,
): Mapped {
    // map each own entry under its key
    const mapped: Record<string, unknown> = {};
    for (const key of Object.keys(source)) {
        const property = key as keyof Source & string;
        const value = map(source[property], property);

        // define a `__proto__` key, which assignment would take as the prototype
        if (key === "__proto__") {
            Object.defineProperty(mapped, key, {
                value,
                enumerable: true,
                writable: true,
                configurable: true,
            });
        }
        // assign any other key
        else {
            mapped[key] = value;
        }
    }

    return mapped as Mapped;
}
