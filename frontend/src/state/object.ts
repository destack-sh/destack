import { graphql } from "@/gql";
import { TypeTag, type RemoteObject, RemoteObjectStatus } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { useApolloClient } from "@vue/apollo-composable";

export const OBJECT_TYPETAGS = [TypeTag.File, TypeTag.Image, TypeTag.Audio, TypeTag.Video];

// :RemoteObjectType
export type ObjectRecord = Pick<RemoteObject, "id" | "status" | "name" | "contentType" | "contentLength" | "sha512">;

export function toObjectDataId(id: string) {
  /* From btoa encoded RemoteObject:uuid to uuid */
  return atob(id).split(":")[1];
}

export function toRemoteObjectId(id: string) {
  /* From uuid to btoa encoded RemoteObject:uuid */
  return btoa(`RemoteObject:${id}`);
}

function makeBasicObject(remoteObject: RemoteObject, status?: RemoteObjectStatus): ObjectRecord {
  return {
    id: toObjectDataId(remoteObject.id),
    name: remoteObject.name,
    contentLength: remoteObject.contentLength,
    contentType: remoteObject.contentType,
    sha512: remoteObject.sha512,
    status: status ?? remoteObject.status,
  };
}

export function humanizeBytes(bytes: number) {
  /** Shorten bytes into nearest (KB, MB, GB, etc.) */
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let unit = 0;
  while (bytes >= 1024 && unit < units.length - 1) {
    bytes /= 1024;
    unit++;
  }
  return `${bytes.toFixed(1)}${units[unit]}`;
}

export function useObjects() {
  const ops = useOperations();
  const apollo = useApolloClient();

  async function upload(projectId: string, file: File, updateValue: (value: ObjectRecord | null) => void) {
    const ret = await ops.object.requestUpload(projectId, file);
    if (ret?.data?.requestUploadObject.__typename != "RemoteObject") {
      return; // ops errors are auto-handled
    }
    const remoteObject = ret.data.requestUploadObject;
    if (remoteObject.status == RemoteObjectStatus.Available) {
      updateValue(makeBasicObject(remoteObject));
      return; // already uploaded
    }
    if (remoteObject.presignedPost == null) {
      throw new Error("no presigned post on remote object");
    }
    // emit uploading state
    updateValue(makeBasicObject(remoteObject, RemoteObjectStatus.Uploading));
    await ops.object.doUpload(remoteObject.id, remoteObject.presignedPost, file);
    await ops.object.notifyUploaded(remoteObject.id);
    // emit uploaded state
    updateValue(makeBasicObject(remoteObject, RemoteObjectStatus.Available));
  }

  async function getPresignedGet(objectId: string): Promise<string> {
    /* Fetch the remote object by id (incl. presigned get field) */
    const ret = await apollo.client.query({
      query: graphql(/* GraphQL */ `
        query remoteObject($id: GlobalID!) {
          remoteObject(id: $id) {
            ... on RemoteObject {
              id
              presignedGet
            }
          }
        }
      `),
      variables: { id: toRemoteObjectId(objectId) },
    });
    if (ret.data.remoteObject?.__typename != "RemoteObject") {
      throw new Error("could not get remote object");
    }
    if (ret.data.remoteObject.presignedGet == null) {
      throw new Error("no presigned get on remote object");
    }
    return ret.data.remoteObject.presignedGet;
  }

  return {
    upload,
    getPresignedGet,
  };
}
