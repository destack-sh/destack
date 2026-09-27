import { S3Error } from "./error.ts";

/** Encode a value as SigV4 does: every byte except letters, digits and `-._~`, and slashes in paths. */
export function encodeUri(value: string, isPath: boolean): string {
    // escape the reserved characters encodeURIComponent leaves unescaped
    const encoded = encodeURIComponent(value).replace(
        /[!'()*]/g,
        (character) => `%${character.charCodeAt(0).toString(16).toUpperCase()}`,
    );

    return isPath ? encoded.replaceAll("%2F", "/") : encoded;
}

/** Decode a percent-encoded URI component, refusing malformed UTF-8. */
export function decodeUri(value: string): string {
    try {
        return decodeURIComponent(value);
    } catch (cause) {
        throw new S3Error("InvalidURI", "the request URI is not valid percent-encoded UTF-8", {
            cause,
        });
    }
}

/** Read the query parameters of a URL in order, decoding plus signs as themselves as S3 does. */
export function readQuery(url: URL): [string, string][] {
    // split the raw query and decode each name and value
    const search = url.search.slice(1);
    if (search === "") {
        return [];
    }

    return search.split("&").map((pair) => {
        // split each pair at its first equals sign
        const separator = pair.indexOf("=");
        const name = separator === -1 ? pair : pair.slice(0, separator);
        const value = separator === -1 ? "" : pair.slice(separator + 1);

        return [decodeUri(name), decodeUri(value)];
    });
}
