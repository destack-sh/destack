import type { Blob, Connection, Workspace } from "../../src/index.js";
import { Destack } from "../../src/index.js";

declare const connection: Connection;
declare const blob: Blob;
declare const stream: AsyncIterable<Uint8Array>;

const destack = new Destack(connection);
const remote: Promise<Destack> = Destack.connect("ws://localhost/rpc");
const workspace: Promise<Workspace> = destack.daemon.openWorkspace("/project");
const uploaded: Promise<Blob> = destack.blobs.put(stream);
const chunks: AsyncGenerator<Uint8Array> = destack.blobs.read(blob, {
    offset: 1n,
    byteLen: 2n,
});
const bytes: Promise<Uint8Array> = destack.blobs.bytes(blob);
const isPresent: Promise<boolean> = destack.blobs.contains(blob);

destack.daemon.shutdown();
destack.close();

void remote;
void workspace;
void uploaded;
void chunks;
void bytes;
void isPresent;
