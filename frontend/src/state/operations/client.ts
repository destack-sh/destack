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
        $projectVersionId: GlobalID
      ) {
        upsertClient(
          input: {
            id: $id
            type: $type
            deviceName: $deviceName
            browserName: $browserName
            projectVersionId: $projectVersionId
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
    projectVersionId: string | null
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
          projectVersionId,
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

  async function updatePresence() {
    return await operations.perform({
      type: "client.updatePresence",
      stateless: true,
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
