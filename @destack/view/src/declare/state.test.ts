import { expect, test } from "@destack/test";
import { ViewState } from "./state.ts";

/** Answer a refusal as the machine does: its failure as JSON, with its headers. */
function refused(status: number, reason: string, message: string, headers: HeadersInit = {}) {
    const failure = {
        defined: true,
        code: "SERVICE_UNAVAILABLE",
        status,
        message,
        reason,
        domain: "@destack/stack",
    };

    return Response.json(failure, { status, headers });
}

/** Request a page as a browser marks it, by its fetch mode and destination. */
function requested(mode: string, destination: string): Request {
    return new Request("http://notes--home--ada.localhost/", {
        headers: { "sec-fetch-mode": mode, "sec-fetch-dest": destination },
    });
}

/** Read a page's status, the headers it keeps and its whole document. */
async function read(response: Response) {
    return {
        status: response.status,
        headers: Object.fromEntries(response.headers),
        body: await response.text(),
    };
}

test("answer a refused page load with its state's page, a starting app reloading", async () => {
    // refuse a page load while the app starts, and for a missing view
    const starting = await ViewState.document(
        refused(
            503,
            "INSTALLATION_UNAVAILABLE",
            "installation 'home' is unavailable: this space does not serve it yet",
            {
                "retry-after": "1",
            },
        ),
    );
    const missing = await ViewState.document(
        refused(404, "VIEW_NOT_FOUND", "home mounts no view x"),
    );

    expect({
        starting: await read(starting),
        missing: await read(missing),
    }).toMatchInlineSnapshot(`
      {
        "missing": {
          "body": "<!doctype html>
      <html>
      <head>
      <meta charset="utf-8">
      <meta name="viewport" content="width=device-width, initial-scale=1">
      <meta name="color-scheme" content="light dark">
      <title>Not found</title>
      <style>
      body { margin: 0; min-height: 100vh; display: grid; place-items: center; font: 15px/1.5 system-ui, sans-serif; background: Canvas; color: CanvasText; }
      main { max-width: 32rem; padding: 2rem; text-align: center; }
      h1 { margin: 0 0 0.5rem; font-size: 1.25rem; font-weight: 600; }
      p { margin: 0; opacity: 0.7; }
      </style>
      </head>
      <body>
      <main data-state="not-found">
      <h1>Not found</h1>
      <p>home mounts no view x</p>
      </main>
      </body>
      </html>
      ",
          "headers": {
            "content-type": "text/html; charset=utf-8",
          },
          "status": 404,
        },
        "starting": {
          "body": "<!doctype html>
      <html>
      <head>
      <meta charset="utf-8">
      <meta name="viewport" content="width=device-width, initial-scale=1">
      <meta name="color-scheme" content="light dark">
      <meta http-equiv="refresh" content="1">
      <title>Starting…</title>
      <style>
      body { margin: 0; min-height: 100vh; display: grid; place-items: center; font: 15px/1.5 system-ui, sans-serif; background: Canvas; color: CanvasText; }
      main { max-width: 32rem; padding: 2rem; text-align: center; }
      h1 { margin: 0 0 0.5rem; font-size: 1.25rem; font-weight: 600; }
      p { margin: 0; opacity: 0.7; }
      </style>
      </head>
      <body>
      <main data-state="starting">
      <h1>Starting…</h1>
      <p>installation 'home' is unavailable: this space does not serve it yet</p>
      </main>
      </body>
      </html>
      ",
          "headers": {
            "content-type": "text/html; charset=utf-8",
            "retry-after": "1",
          },
          "status": 503,
        },
      }
    `);
});

test("tell a page load from other requests, as browsers mark a document navigation", () => {
    // load a page, fetch from one, and load a frame
    expect([
        ViewState.isPageLoad(requested("navigate", "document")),
        ViewState.isPageLoad(requested("cors", "empty")),
        ViewState.isPageLoad(requested("navigate", "iframe")),
    ]).toEqual([true, false, false]);
});
