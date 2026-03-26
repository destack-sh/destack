/** Read the first request body value from form style input. */
export function readRequestBody(value: string | string[]) {
    if (typeof value === "string") {
        return value;
    }

    const bodyValue = value[0];
    if (bodyValue === undefined) {
        return "";
    }

    return bodyValue;
}
