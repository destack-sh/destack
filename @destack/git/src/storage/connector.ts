import type { Connector } from "@destack/resource";
import type { GitStorageDeclaration } from "./kind.ts";
import type { GitStorage } from "./storage.ts";

/** The connectors opening Git storages declared alone: none, as every host supplies its own. */
export const connectors: Readonly<Record<string, Connector<GitStorage, GitStorageDeclaration>>> =
    {};
