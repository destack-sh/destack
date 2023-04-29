import type { ClientType } from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";
import { graphql } from "@/gql";

export function useClientOps() {
  const operations = useOperationsStore();

  const { mutate: upsertClientMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation upsertClient(
        $id: GlobalID!
        $type: ClientType!
        $deviceName: String
        $browserName: String
        $projectId: GlobalID
        $projectVersionId: GlobalID
        $fileId: GlobalID
        $statementId: GlobalID
        $typeNodeId: GlobalID
        $recordId: GlobalID
        $path: String
      ) {
        upsertClient(
          input: {
            id: $id
            type: $type
            deviceName: $deviceName
            browserName: $browserName
            projectId: $projectId
            projectVersionId: $projectVersionId
            fileId: $fileId
            statementId: $statementId
            typeNodeId: $typeNodeId
            recordId: $recordId
            path: $path
          }
        ) {
          ... on Client {
            id
            type
            deviceName
            browserName
            projectVersion {
              id
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function upsert(
    id: string,
    type: ClientType,
    deviceName: string,
    browserName: string,
    projectId: string | null,
    projectVersionId: string | null,
    fileId: string | null,
    statementId: string | null,
    typeNodeId: string | null,
    recordId: string | null,
    path: string | null
  ) {
    return await operations.perform({
      type: "client.upsert",
      stateless: true,
      do: async () => {
        return await upsertClientMut({
          id,
          type,
          deviceName,
          browserName,
          projectId,
          projectVersionId,
          fileId,
          statementId,
          typeNodeId,
          recordId,
          path,
        });
      },
    });
  }

  const { mutate: closeClientMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation closeClient {
        closeClient {
          ...OperationInfoContent
        }
      }
    `)
  );

  async function close() {
    return await operations.perform({
      type: "client.close",
      stateless: true,
      suppressErrors: true,
      do: async () => {
        return await closeClientMut();
      },
    });
  }

  const { mutate: updatePresenceMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updatePresence {
        updatePresence {
          ...OperationInfoContent
        }
      }
    `)
  );

  async function updatePresence(silent?: boolean) {
    return await operations.perform({
      type: "client.updatePresence",
      stateless: true,
      suppressErrors: silent,
      do: async () => {
        return await updatePresenceMut();
      },
    });
  }

  return {
    upsert,
    close,
    updatePresence,
  };
}
