/** Order text independent of the host locale. */
export const Text = {
    /** Order two strings by UTF-16 code units. */
    compare(left: string, right: string): number {
        return Number(left > right) - Number(left < right);
    },
};
