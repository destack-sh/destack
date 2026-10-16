import type { Connector } from "@destack/resource";
import type { GitStorageDeclaration } from "./kind.ts";
import type { GitStorage } from "./storage.ts";

/** The connectors opening Git storages where the host supplies them, such as workerd: none. */
export const connectors: Readonly<Record<string, Connector<GitStorage, GitStorageDeclaration>>> =
    {};
