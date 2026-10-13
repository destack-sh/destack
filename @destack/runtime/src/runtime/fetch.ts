/** Send a request and answer its response, as the web platform's fetch does. */
export type Fetch = (request: Request) => Promise<Response>;
