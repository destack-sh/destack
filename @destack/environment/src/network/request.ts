/** Copy a request with changes and an optional URL. */
export function copyRequest(request: Request, changes: RequestInit, url = request.url): Request {
    // keep the body's source on the same URL, so a browser sends it whole, never as a stream upload
    if (url === request.url && changes.body === undefined) {
        // copy it, dropping each header the given ones lack
        // NOTE #Robustness: Bun keeps the request's own headers when the given set is empty
        const copied = new Request(request, changes);
        if (changes.headers !== undefined) {
            const given = new Headers(changes.headers);
            for (const name of [...copied.headers.keys()].filter((each) => !given.has(each))) {
                copied.headers.delete(name);
            }
        }

        return copied;
    }

    // stream a body on to another URL as half duplex
    const body = changes.body === undefined ? request.body : changes.body;

    return new Request(url, {
        method: changes.method ?? request.method,
        headers: changes.headers ?? request.headers,
        body,
        signal: changes.signal ?? request.signal,
        redirect: changes.redirect ?? request.redirect,
        ...(body instanceof ReadableStream ? { duplex: "half" } : {}),
    });
}
