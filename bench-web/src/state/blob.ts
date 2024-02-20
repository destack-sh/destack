import { graphql } from "@/gql";
import { BlobStatus, TypeTag, type Blob } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { BLOB_TYPENAME } from "@/state/type";
import { useApolloClient } from "@vue/apollo-composable";

export const OBJECT_TYPETAGS = [TypeTag.Blob];

// :BlobType
export type BlobRecord = {
  _type: typeof BLOB_TYPENAME;
  id: string;
  name?: string | null;
  content_length: number;
  content_type: string;
  sha512: string;
  status: BlobStatus;
};

const OBJECT_RECORD_FIELD_TYPES: Record<string, string> = {
  id: "string",
  content_length: "number",
  content_type: "string",
  sha512: "string",
};

export function toObjectDataId(id: string) {
  /* From btoa encoded Blob:uuid to uuid */
  return atob(id).split(":")[1];
}

export function toBlobId(id: string) {
  /* From uuid to btoa encoded Blob:uuid */
  return btoa(`Blob:${id}`);
}

function makeBlob(blob: Blob, status?: BlobStatus): BlobRecord {
  const record = {
    _type: BLOB_TYPENAME,
    id: toObjectDataId(blob.id),
    name: blob.name ?? null,
    content_length: blob.contentLength,
    content_type: blob.contentType,
    sha512: blob.sha512,
    status: status ?? blob.status,
  } as BlobRecord;
  if (!isValidBlobRecord(record)) {
    throw new Error("invalid object record");
  }
  return record;
}

export function isValidBlobRecord(obj: any): boolean {
  if (obj?._type != BLOB_TYPENAME) {
    return false;
  }
  // check required field types
  for (const field of Object.keys(OBJECT_RECORD_FIELD_TYPES)) {
    if (typeof obj[field] != OBJECT_RECORD_FIELD_TYPES[field]) {
      return false;
    }
  }
  return true;
}

export function humanizeBytes(bytes: number, options?: { round?: boolean }) {
  /** Shorten bytes into nearest (KB, MB, GB, etc.), keep up to 3 significant digits */
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let unit = 0;
  while (bytes >= 100 && unit < units.length - 1) {
    bytes /= 1024;
    unit++;
  }
  if (options?.round) {
    return `${Math.round(bytes)}${units[unit]}`;
  }
  if (unit == 0) {
    return `${bytes.toFixed(0)}${units[unit]}`;
  } else if (bytes < 10) {
    return `${bytes.toFixed(1)}${units[unit]}`;
  } else {
    return `${bytes.toFixed(0)}${units[unit]}`;
  }
}

export function useObjects() {
  const ops = useOperations();
  const apollo = useApolloClient();

  async function upload(projectId: string, file: File, updateValue: (value: BlobRecord | null) => void) {
    const ret = await ops.object.requestUpload(projectId, file);
    if (ret?.data?.requestUploadObject.__typename != "Blob") {
      return; // ops errors are auto-handled
    }
    const blob = ret.data.requestUploadObject;
    if (blob.status == BlobStatus.Available) {
      updateValue(makeBlob(blob));
      return; // already uploaded
    }
    if (blob.presignedPost == null) {
      throw new Error("no presigned post on blob");
    }
    // don't emit uploading state since that would cause an extra state change
    // (which is meaningless to undo but too far down the pipe to easily bind to a tx)
    await ops.object.doUpload(blob.id, blob.presignedPost, file);
    await ops.object.notifyUploaded(blob.id);
    // emit uploaded state
    updateValue(makeBlob(blob, BlobStatus.Available));
  }

  async function getPresignedGet(objectId: string): Promise<string> {
    /* Fetch the blob by id (incl. presigned get field) */
    const ret = await apollo.client.query({
      query: graphql(/* GraphQL */ `
        query blob($id: GlobalID!) {
          blob(id: $id) {
            ... on Blob {
              id
              presignedGet
            }
          }
        }
      `),
      variables: { id: toBlobId(objectId) },
    });
    if (ret.data.blob?.__typename != "Blob") {
      throw new Error("could not GET blob");
    }
    if (ret.data.blob.presignedGet == null) {
      throw new Error("no presigned GET on blob");
    }
    return ret.data.blob.presignedGet;
  }

  return {
    upload,
    getPresignedGet,
  };
}
