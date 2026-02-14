type Value = First | // union-line
    Second | Third;

const config = { retries: 3 } satisfies // sat-tail
Record<string, number>;

const value = source as // as-tail
number;
