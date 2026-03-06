// fixture server: minimal stdio json rpc server for bridge smoke tests
let inputBuffer = Buffer.alloc(0);

// append new bytes and parse complete frames
process.stdin.on("data", (chunk) => {
    inputBuffer = Buffer.concat([inputBuffer, chunk]);
    drainMessages();
});

/** Drain complete LSP frames from buffered stdin. */
function drainMessages() {
    // parse full lsp frames from the buffered stream
    while (true) {
        const headerEnd = inputBuffer.indexOf("\r\n\r\n");
        if (headerEnd < 0) {
            return;
        }

        const header = inputBuffer.subarray(0, headerEnd).toString("utf8");
        const contentLength = parseContentLength(header);
        if (contentLength === undefined) {
            inputBuffer = inputBuffer.subarray(headerEnd + 4);
            continue;
        }

        const messageEnd = headerEnd + 4 + contentLength;
        if (inputBuffer.length < messageEnd) {
            return;
        }

        const payload = inputBuffer.subarray(headerEnd + 4, messageEnd).toString("utf8");
        inputBuffer = inputBuffer.subarray(messageEnd);

        try {
            const message = JSON.parse(payload);
            handleMessage(message);
        } catch {
            // ignore malformed frames in this fixture process
        }
    }
}

/** Parse one content-length header value. */
function parseContentLength(header) {
    // extract content length from the lsp header block
    const match = /Content-Length:\s*(\d+)/i.exec(header);
    if (!match) {
        return undefined;
    }

    return Number.parseInt(match[1], 10);
}

/** Write one JSON-RPC payload with LSP framing. */
function sendMessage(message) {
    // frame and write one json rpc payload
    const body = Buffer.from(JSON.stringify(message), "utf8");
    process.stdout.write(`Content-Length: ${body.length}\r\n\r\n`);
    process.stdout.write(body);
}

/** Send one JSON-RPC success response. */
function sendResponse(id, result) {
    // send standard response shape
    sendMessage({
        jsonrpc: "2.0",
        id,
        result,
    });
}

/** Publish deterministic diagnostics from one source text payload. */
function sendPublishDiagnostics(uri, text) {
    // synthesize one deterministic diagnostic pattern
    const hasError = /export const\s+\w+\s*=\s*;/.test(text);
    const diagnostics = hasError
        ? [
              {
                  range: {
                      start: { line: 0, character: 0 },
                      end: { line: 0, character: 1 },
                  },
                  severity: 1,
                  source: "destack-fixture",
                  message: "fixture parse error",
              },
          ]
        : [];

    sendMessage({
        jsonrpc: "2.0",
        method: "textDocument/publishDiagnostics",
        params: {
            uri,
            diagnostics,
        },
    });
}

/** Handle one inbound request payload. */
function handleRequest(message) {
    const method = message.method;
    const id = message.id;

    // initialize: advertise required smoke-test capabilities
    if (method === "initialize") {
        sendResponse(id, {
            capabilities: {
                textDocumentSync: 1,
                definitionProvider: true,
                hoverProvider: true,
                executeCommandProvider: {
                    commands: ["destack.rescan", "destack.reindex", "destack.clearCache"],
                },
            },
            serverInfo: {
                name: "destack-bridge-fixture",
                version: "1.0.0",
            },
        });
        return;
    }

    // shutdown: acknowledge and wait for exit
    if (method === "shutdown") {
        sendResponse(id, null);
        return;
    }

    // definition: return one stable same-file location
    if (method === "textDocument/definition") {
        const uri = message.params?.textDocument?.uri;
        if (!uri) {
            sendResponse(id, []);
            return;
        }

        sendResponse(id, [
            {
                uri,
                range: {
                    start: { line: 0, character: 16 },
                    end: { line: 0, character: 22 },
                },
            },
        ]);
        return;
    }

    // hover: return one stable payload
    if (method === "textDocument/hover") {
        sendResponse(id, {
            contents: {
                kind: "plaintext",
                value: "mock hover",
            },
        });
        return;
    }

    // execute command: success response only
    if (method === "workspace/executeCommand") {
        sendResponse(id, null);
        return;
    }

    // pull diagnostics: return one empty report
    if (method === "textDocument/diagnostic") {
        sendResponse(id, { kind: "full", items: [] });
        return;
    }

    // workspace diagnostics: return one empty aggregate report
    if (method === "workspace/diagnostic") {
        sendResponse(id, { items: [] });
        return;
    }

    // unknown request: reply with null result
    sendResponse(id, null);
}

/** Handle one inbound notification payload. */
function handleNotification(message) {
    const method = message.method;

    // didOpen: publish diagnostics from full text content
    if (method === "textDocument/didOpen") {
        const uri = message.params?.textDocument?.uri;
        const text = message.params?.textDocument?.text ?? "";
        if (uri) {
            sendPublishDiagnostics(uri, text);
        }
        return;
    }

    // didChange: publish diagnostics from latest full text change
    if (method === "textDocument/didChange") {
        const uri = message.params?.textDocument?.uri;
        const changes = message.params?.contentChanges ?? [];
        const latest = changes.length > 0 ? (changes[changes.length - 1].text ?? "") : "";
        if (uri) {
            sendPublishDiagnostics(uri, latest);
        }
        return;
    }

    // exit: terminate the fixture process
    if (method === "exit") {
        process.exit(0);
    }
}

/** Dispatch one inbound JSON-RPC payload by shape. */
function handleMessage(message) {
    // route request and notification payloads
    if (Object.hasOwn(message, "id")) {
        handleRequest(message);
        return;
    }

    handleNotification(message);
}
