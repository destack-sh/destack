import { testHost } from "@destack/test/host";
import { testBuild } from "@destack/object/test";
import { TestPackage } from "@destack/stack/test";
import connectionPackage from "../package.ts";
import connectionServer from "../server.ts";

/** Connections as the machine ships them, beside the vault keeping their credentials. */
export const CONNECTION: TestPackage = {
    handle: connectionPackage,
    build: await testBuild(connectionPackage, testHost().crypto),
    server: connectionServer,
    connectors: [],
    providers: [],
};
