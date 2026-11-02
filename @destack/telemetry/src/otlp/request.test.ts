import { expect, refusal, test } from "@destack/test";
import { OtlpExport } from "./request.ts";

/** The installation a forwarder knows an export comes from. */
const INSTALLATION = "installation-01996ab0-0000-7000-8000-000000000001";

/** The build manifest a forwarder knows an export's installation serves. */
const MANIFEST = "5f8d1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7";

test("stamp each resource of an export with its forwarder's emitter, refusing an export claiming one", async () => {
    // stamp a resource with its own attributes, and one without any
    const own = { key: "service.name", value: { stringValue: "notes" } };
    const stamped = new OtlpExport("logs", {
        resourceLogs: [{ resource: { attributes: [own] }, scopeLogs: [] }, { scopeLogs: [] }],
    }).stamp({ installation: INSTALLATION, build: MANIFEST }).body;

    // refuse a resource claiming its build and another installation
    const forged = [
        { key: "destack.build.manifest", value: { stringValue: MANIFEST } },
        { key: "destack.installation.id", value: { stringValue: "installation-forged" } },
    ];
    const claim = () =>
        new OtlpExport("logs", {
            resourceLogs: [{ resource: { attributes: [own, ...forged] }, scopeLogs: [] }],
        }).stamp({ installation: INSTALLATION });

    // set the known emitter on each resource after its other attributes, and refuse the claim
    const known = [
        { key: "destack.installation.id", value: { stringValue: INSTALLATION } },
        { key: "destack.build.manifest", value: { stringValue: MANIFEST } },
    ];
    expect(stamped).toEqual({
        resourceLogs: [
            { resource: { attributes: [own, ...known] }, scopeLogs: [] },
            { resource: { attributes: known }, scopeLogs: [] },
        ],
    });
    expect(await refusal(Promise.resolve().then(claim))).toEqual([
        "FORBIDDEN",
        "EMITTER_ATTRIBUTE_RESERVED",
        "only the receiver can record an export's emitter, found destack.build.manifest, destack.installation.id",
    ]);
});
