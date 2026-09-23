import { AuditRecorder } from "@destack/audit";
import { daemonPackage } from "../audit/index.ts";
import { expect, test } from "@destack/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DaemonStore } from "./store.ts";

test("preserve host identity and audit after reopen", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-host-"));
    try {
        // retain the identity created by a fresh database
        const initial = await DaemonStore.open(directory);
        const identity = await initial.host.rename(
            "integration host",
            new AuditRecorder(
                {
                    actor: { type: "system", name: "fixture" },
                    delegation: [],
                    package: daemonPackage,
                    service: "daemon",
                    hostId: (await initial.host.get()).hostId,
                },
                initial.outbox,
            ),
        );
        await initial.close();

        // reopen without duplicating identity or audit records
        for (let index = 0; index < 2; index++) {
            const storage = await DaemonStore.open(directory);
            try {
                expect(await storage.host.get()).toEqual(identity);
                const recorded = await storage.outbox.read();
                expect(
                    recorded.map((event) => ({
                        action: event.action.name,
                        targets: event.targets,
                        details: event.details,
                        result: event.result,
                        hostId: event.context.hostId,
                    })),
                ).toEqual([
                    {
                        action: "host.rename",
                        targets: { host: { type: "host", id: identity.hostId } },
                        details: { name: "integration host" },
                        result: { stage: "result", outcome: "success" },
                        hostId: identity.hostId,
                    },
                ]);
            } finally {
                await storage.close();
            }
        }
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
