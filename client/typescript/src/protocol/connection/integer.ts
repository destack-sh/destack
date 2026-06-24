/** Return one checked JS number for a protocol integer. */
export function checkedNumber(value: bigint, name: string): number {
    const number = Number(value);
    if (!Number.isSafeInteger(number) || number < 0) {
        throw new Error(`${name} is outside the safe integer range: ${value}`);
    }

    return number;
}
