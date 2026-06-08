export { detectBackend } from "./backend.js";
export type { LanguageBackend } from "./backend.js";
export type { Revision } from "./repository/revision.generated.js";
export type { SessionFile } from "./session/file.generated.js";
export type { Module } from "./session/module.generated.js";
export { openSession } from "./session/session.js";
export type { OpenSessionInput, Session } from "./session/session.js";
export type {
    FileUpdate,
    FileUpdateKind,
    SourceFile,
    SourceFileContent,
} from "./session/source/file.generated.js";
export type { SourceSnapshot } from "./session/source/snapshot.generated.js";
export type {
    SourceEdit,
    SourceUpdate,
    SourceUpdateResult,
    TextEdit,
    TextRange,
} from "./session/source/update.generated.js";
