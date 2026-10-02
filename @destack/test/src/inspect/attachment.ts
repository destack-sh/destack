import { defineSchema, schema } from "@destack/schema";
import type { TestAnnotation, TestArtifact } from "vitest";
import { Buffer } from "node:buffer";

/** An attachment stored inline or at a path. */
export const AttachmentDescription = defineSchema(
    schema.object({
        contentType: schema.string().exactOptional(),
        path: schema.string().exactOptional(),
        body: schema.string().exactOptional(),
        bodyEncoding: schema.enum(["base64", "utf-8"]).exactOptional(),
    }),
);
/** A portable attachment. */
export type AttachmentDescription = schema.Infer<typeof AttachmentDescription>;

/** A source location including its file. */
export const SourceDescription = defineSchema(
    schema.object({
        file: schema.string(),
        line: schema.number().int(),
        column: schema.number().int(),
    }),
);

/** A test annotation with its optional attachment. */
export const AnnotationDescription = defineSchema(
    schema.object({
        message: schema.string(),
        type: schema.string(),
        location: SourceDescription.exactOptional(),
        attachment: AttachmentDescription.exactOptional(),
    }),
);
/** A portable annotation. */
export type AnnotationDescription = schema.Infer<typeof AnnotationDescription>;

/** An extensible artifact with explicit attachments and JSON properties. */
export const ArtifactDescription = defineSchema(
    schema.object({
        type: schema.string(),
        location: SourceDescription.exactOptional(),
        attachments: schema.array(AttachmentDescription),
        properties: schema.record(schema.string(), schema.json()),
    }),
);
/** A portable test artifact. */
export type ArtifactDescription = schema.Infer<typeof ArtifactDescription>;

/** Describe artifact properties and normalize their standard attachments. */
export function describeArtifact(artifact: TestArtifact): ArtifactDescription {
    const { type, location, attachments, ...properties } = artifact;
    const entries = Object.entries(properties).filter(([, value]) => value !== undefined);

    return ArtifactDescription.parse({
        type,
        location,
        attachments: attachments?.map(describeAttachment) ?? [],
        properties: Object.fromEntries(entries),
    });
}

/** Describe a runner annotation and normalize binary attachments. */
export function describeAnnotation(annotation: TestAnnotation): AnnotationDescription {
    return AnnotationDescription.parse({
        message: annotation.message,
        type: annotation.type,
        location: annotation.location,
        attachment: annotation.attachment && describeAttachment(annotation.attachment),
    });
}

/** Encode binary attachments as base64 without changing text attachments. */
export function describeAttachment(
    attachment: NonNullable<TestAnnotation["attachment"]>,
): AttachmentDescription {
    // encode a binary body as base64 and keep only the fields the attachment has
    const body =
        attachment.body instanceof Uint8Array
            ? Buffer.from(attachment.body).toString("base64")
            : attachment.body;
    const bodyEncoding = attachment.body instanceof Uint8Array ? "base64" : attachment.bodyEncoding;
    const { contentType, path } = attachment;

    return {
        ...(contentType === undefined ? {} : { contentType }),
        ...(path === undefined ? {} : { path }),
        ...(body === undefined ? {} : { body }),
        ...(bodyEncoding === undefined ? {} : { bodyEncoding }),
    };
}
