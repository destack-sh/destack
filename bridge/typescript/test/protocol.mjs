import assert from "node:assert/strict";
import test from "node:test";

import { decodeFrame, encodeFrame } from "../dist/protocol/codec.js";
import { Connection } from "../dist/protocol/connection/index.js";
import {
    protocolLimits,
    protocolVersion,
} from "../dist/generated/protocol/defaults.js";
import { WorkspaceQueryResponse } from "../dist/generated/protocol/query.js";
import { WorkspaceRequest } from "../dist/generated/protocol/request.js";
import { WorkspaceResponse } from "../dist/generated/protocol/response.js";
import { openRemoteWorkspace } from "../dist/protocol/workspace.js";

test("protocol: roundtrips a ping request frame", () => {
    const message = {
        kind: "request",
        request: {
            id: { 0: 7 },
            options: {},
            payload: WorkspaceRequest.ping(),
        },
    };

    // encode through the public frame codec
    const frame = encodeFrame(message);
    const decoded = decodeFrame(frame);

    assert.deepEqual(decoded, message);
});

test("protocol: negotiates a workspace connection", async () => {
    const transport = new MemoryTransport((message) => {
        assert.equal(message.kind, "request");
        assert.equal(message.request.payload.kind, "handshake");

        return {
            kind: "response",
            response: {
                id: message.request.id,
                payload: WorkspaceResponse.handshake({
                    protocol: protocolVersion,
                    server: {
                        name: "test-workspace",
                        version: "0",
                    },
                    limits: protocolLimits,
                }),
            },
        };
    });
    const connection = new Connection(transport);

    const response = await connection.handshake();

    assert.equal(response.server.name, "test-workspace");
    assert.equal(connection.handshakeResponse, response);
    connection.close();
});

test("protocol: opens a remote workspace", async () => {
    const transport = new MemoryTransport((message) => {
        assert.equal(message.kind, "request");

        if (message.request.payload.kind === "handshake") {
            return response(message, WorkspaceResponse.handshake({
                protocol: protocolVersion,
                server: {
                    name: "test-workspace",
                    version: "0",
                },
                limits: protocolLimits,
            }));
        }

        if (message.request.payload.kind === "openRoot") {
            assert.equal(message.request.payload.open_root.root, "/workspace");

            return response(message, WorkspaceResponse.rootOpened({
                handle: { 0: 11 },
                root: "/workspace",
                diagnostics: [],
                messages: [],
            }));
        }

        if (message.request.payload.kind === "query") {
            assert.equal(message.request.payload.query.kind, "currentRevision");

            return response(message, WorkspaceResponse.queryResult(
                WorkspaceQueryResponse.currentRevision({ 0: revisionBytes(3) }),
            ));
        }

        if (message.request.payload.kind === "reloadRoot") {
            assert.equal(message.request.payload.reload_root.reason, "manual");

            return response(message, WorkspaceResponse.rootReloaded({
                handle: { 0: 11 },
                updates: {
                    updates: [],
                    messages: [],
                },
            }));
        }

        if (message.request.payload.kind === "closeRoot") {
            return response(message, WorkspaceResponse.rootClosed({
                handle: { 0: 11 },
            }));
        }

        throw new Error(`unexpected request: ${message.request.payload.kind}`);
    });
    const connection = new Connection(transport);

    const workspace = await openRemoteWorkspace({
        connection,
        workspace: "/workspace",
    });
    const revision = await workspace.revision();
    const reload = await workspace.reload();
    await workspace.close();

    assert.equal(workspace.root(), "/workspace");
    assert.deepEqual(revision, { 0: revisionBytes(3) });
    assert.deepEqual(reload.updates, []);
    connection.close();
});

function response(request, payload) {
    return {
        kind: "response",
        response: {
            id: request.request.id,
            payload,
        },
    };
}

function revisionBytes(byte) {
    return new Uint8Array(32).fill(byte);
}

class MemoryTransport {
    #handler;
    #frames = [];
    #waiters = [];
    #isClosed = false;

    constructor(handler) {
        this.#handler = handler;
    }

    async send(bytes) {
        const request = decodeFrame(bytes);
        const response = this.#handler(request);

        this.#push(encodeFrame(response));
    }

    receive() {
        if (this.#frames.length > 0) {
            return Promise.resolve(this.#frames.shift());
        }

        if (this.#isClosed) {
            return Promise.reject(new Error("transport closed"));
        }

        return new Promise((resolve, reject) => {
            this.#waiters.push({ resolve, reject });
        });
    }

    close() {
        this.#isClosed = true;

        while (this.#waiters.length > 0) {
            this.#waiters.shift().reject(new Error("transport closed"));
        }
    }

    #push(frame) {
        const waiter = this.#waiters.shift();
        if (waiter === undefined) {
            this.#frames.push(frame);
        } else {
            waiter.resolve(frame);
        }
    }
}
