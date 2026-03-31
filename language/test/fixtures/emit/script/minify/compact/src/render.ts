export function renderCompactMessage(name: string, value: string) {
    const prefix = name.toUpperCase();

    return `${prefix}:${value}`;
}
