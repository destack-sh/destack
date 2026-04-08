type Value =
    | First // union-line
    | Second
    | Third;

const config = {
    retries: 3,
} satisfies Record<string, number>; // sat-tail

const value = source as number; // as-tail
