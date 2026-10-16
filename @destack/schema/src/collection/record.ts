/** Build a record from an object's own keys, typed as the mapped type its caller states. */
export function mapValues<Source extends object, Mapped extends object>(
    source: Source,
    map: (value: Source[keyof Source], key: keyof Source & string) => unknown,
): Mapped {
    // map each own entry under its key
    const entries = Object.keys(source).map((key) => {
        const property = key as keyof Source & string;

        return [key, map(source[property], property)];
    });

    return Object.fromEntries(entries) as Mapped;
}
