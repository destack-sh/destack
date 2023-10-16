import { graphql } from "@/gql";
import { BlobStatus } from "@/gql/graphql";
import type { ObjectRecord } from "@/state/object";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useObjectOps() {
  const ops = useOperationsStore();

  const { mutate: requestUploadObjectMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation requestUploadObject(
        $projectId: GlobalID!
        $name: String
        $contentType: String!
        $contentLength: Int!
        $sha512: String!
      ) {
        requestUploadObject(
          input: {
            projectId: $projectId
            name: $name
            contentType: $contentType
            contentLength: $contentLength
            sha512: $sha512
          }
        ) {
          ... on Blob {
            id
            status
            name
            contentType
            contentLength
            sha512
            presignedPost
            presignedGet
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function requestUploadObject(
    projectId: string,
    name: string,
    contentType: string,
    contentLength: number,
    sha512: string
  ) {
    return await ops.perform({
      type: "object.requestUpload",
      stateless: true,
      do: async () => {
        return await requestUploadObjectMut({
          projectId,
          name,
          contentType,
          contentLength,
          sha512,
        });
      },
    });
  }

  async function prepareUpload(projectId: string, file: File): Promise<Omit<ObjectRecord, "id">> {
    const sha512 = await computeSHA512(file);
    return {
      __typename: "Blob",
      name: file.name,
      content_type: file.type,
      content_length: file.size,
      sha512,
      status: BlobStatus.Prepared,
    };
  }

  async function requestUpload(projectId: string, file: File) {
    const sha512 = await computeSHA512(file);
    const { name, type, size } = file;
    return await requestUploadObject(projectId, name, type, size, sha512);
  }

  const { mutate: notifyUploadedObjectMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation notifyUploadedObject($id: GlobalID!) {
        notifyUploadedObject(input: { id: $id }) {
          ... on Blob {
            id
            status
            name
            contentType
            contentLength
            sha512
            presignedGet
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function notifyUploaded(id: string) {
    return await ops.perform({
      type: "object.notifyUploaded",
      stateless: true,
      do: async () => {
        return await notifyUploadedObjectMut({ id });
      },
    });
  }

  async function doUpload(objectId: string, presignedPostUrl: string, file: File) {
    /** Actually upload the given file to the pre-signed URL with POST multipart/form-data */
    const formData = new FormData();
    // strip parameters from the URL and add to the form data (not sure why this is necessary?)
    const url = new URL(presignedPostUrl);
    for (const [key, value] of url.searchParams.entries()) {
      formData.append(key, value);
    }
    formData.append("file", file); // must be last
    try {
      const urlMain = url.origin + url.pathname;
      const rep = await fetch(urlMain, {
        method: "POST",
        body: formData,
      });
      if (!rep.ok) {
        throw new Error(`failed to upload file: ${rep.status} ${rep.statusText}`);
      }
    } catch (e) {
      console.error(`failed to upload file: ${e}`, e);
      throw e;
    }
  }

  async function getUrl(objectId: string) {
    /** Get the presigned GET URL for the given object */
    throw new Error("not implemented");
  }

  return {
    requestUpload,
    prepareUpload,
    notifyUploaded,
    doUpload,
    getUrl,
  };
}

function computeSHA512(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const arrayBuffer = reader.result as ArrayBuffer;
      crypto.subtle
        .digest("SHA-512", arrayBuffer)
        .then((hashBuffer) => {
          const hashArray = Array.from(new Uint8Array(hashBuffer));
          const hashHex = hashArray.map((byte) => byte.toString(16).padStart(2, "0")).join("");
          resolve(hashHex);
        })
        .catch((error) => {
          reject(error);
        });
    };
    reader.onerror = (error) => {
      reject(error);
    };
    reader.readAsArrayBuffer(file);
  });
}
