import type * as Napi from "@destack/language-napi";
import type { SessionFile } from "../session/file.generated.js";
import type { Module } from "../session/module.generated.js";
import type { FileUpdate } from "../session/source/file.generated.js";
import type { SourceSnapshot } from "../session/source/snapshot.generated.js";
import type { SourceUpdate, SourceUpdateResult } from "../session/source/update.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import type { Session } from "../session/session.js";
import {
    fromNapiFileUpdate,
    fromNapiModule,
    fromNapiSessionFile,
    fromNapiSourceUpdateResult,
    toNapiSourceSnapshot,
    toNapiSourceUpdate,
} from "./generated.js";

type NapiModule = typeof import("@destack/language-napi");

/** Open one NAPI language session from a native filesystem path. */
export async function openNapiPath(path: string): Promise<Session> {
    const napi = await import("@destack/language-napi");
    const session = napi.Session.openPath(path);

    return new NativeSession(session);
}

/** Open one NAPI language session from an explicit source snapshot. */
export async function openNapiSource(root: string, source: SourceSnapshot): Promise<Session> {
    const napi = await import("@destack/language-napi");
    const session = napi.Session.openSource(root, toNapiSourceSnapshot(source));

    return new NativeSession(session);
}

class NativeSession implements Session {
    public readonly backend = "napi";

    public constructor(private readonly session: InstanceType<NapiModule["Session"]>) {}

    public revision(): Revision {
        return this.session.revision();
    }

    public files(): readonly SessionFile[] {
        return this.session.files().map(fromNapiSessionFile);
    }

    public update(update: SourceUpdate): SourceUpdateResult {
        const result = this.session.update(toNapiSourceUpdate(update));

        return fromNapiSourceUpdateResult(result);
    }

    public reload(): readonly FileUpdate[] {
        return this.session.reload().map(fromNapiFileUpdate);
    }

    public loadModule(path: string): Module {
        return fromNapiModule(this.session.loadModule(path));
    }
}
