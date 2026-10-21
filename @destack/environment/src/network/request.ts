/** Copy a request with changes and an optional URL. */
export function copyRequest(request: Request, changes: RequestInit, url = request.url): Request {
    // stream a body on as half duplex
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
