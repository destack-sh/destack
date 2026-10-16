import type { Connector } from "@destack/resource";
import type { GitStorageDeclaration } from "../storage/kind.ts";
import type { GitStorage } from "../storage/storage.ts";
import { LocalGitStorage } from "./storage.ts";

/** The connectors opening Git storages on Bun: local directories. */
export const connectors: Readonly<Record<string, Connector<GitStorage, GitStorageDeclaration>>> = {
    [LocalGitStorage.connector.code]: LocalGitStorage.connector,
};
