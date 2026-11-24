/** The units a size is written in, each a thousand of the one before. */
const SIZE_UNITS = ["byte", "kilobyte", "megabyte", "gigabyte", "terabyte"] as const;

/** How many of a unit make the next. */
const SIZE_STEP = 1000;

/** Sizes of files in bytes. */
export const FileSize = {
    /** Write a size in the largest unit it has one of once rounded, such as 2.4 MB. */
    write(
        bytes: number,
        write: (value: number, options: Intl.NumberFormatOptions) => string,
    ): string {
        // round in each unit, and take the first that stays below the next
        const last = SIZE_UNITS.length - 1;
        const at = SIZE_UNITS.findIndex(
            (_, place) => place === last || rounded(bytes / SIZE_STEP ** place, place) < SIZE_STEP,
        );

        return write(rounded(bytes / SIZE_STEP ** at, at), {
            style: "unit",
            unit: SIZE_UNITS[at] ?? "byte",
            unitDisplay: at === 0 ? "long" : "short",
            maximumFractionDigits: at === 0 ? 0 : 1,
        });
    },
};

/** Round a size whole in bytes, and to a tenth in larger units. */
function rounded(size: number, place: number): number {
    return place === 0 ? Math.round(size) : Math.round(size * 10) / 10;
}
