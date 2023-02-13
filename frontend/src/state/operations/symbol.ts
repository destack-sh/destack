import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolContentOps() {
  const operations = useOperationsStore();

  // type mutations

  const { mutate: updateStatementDescriptionMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateStatementDescription($id: GlobalID!, $description: String!) {
        updateStatementDescription(input: { id: $id, description: $description }) {
          ... on Statement {
            id
            description
            revision
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function updateStatementDescription(id: string, oldDescription: string, newDescription: string) {
    await operations.perform({
      type: "statement.updateDescription",
      do: async () => {
        await updateStatementDescriptionMut({ id: id, description: newDescription });
      },
      undo: async () => {
        await updateStatementDescriptionMut({ id: id, description: oldDescription });
      },
    });
  }

  // code mutations

  const { mutate: updateStatementCodeMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateStatementCode($id: GlobalID!, $code: String) {
        updateStatementCode(input: { id: $id, code: $code }) {
          ... on Statement {
            id
            code
            revision
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function updateStatementCode(id: string, oldCode: string, newCode: string) {
    await operations.perform({
      type: "statement.updateCode",
      do: async () => {
        await updateStatementCodeMut({
          id: id,
          code: newCode,
        });
      },
      undo: async () => {
        await updateStatementCodeMut({
          id: id,
          code: oldCode,
        });
      },
    });
  }

  // dataset mutations (aka records)

  // TODO @Performance: mutate records optimistically
  const { mutate: createRecordMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {
        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {
          ... on Statement {
            id
            orderKey
            revision
            records {
              id
              orderKey
              data
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  const { mutate: updateRecordMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {
        updateStatementRecord(input: { id: $id, statementId: $statementId, data: $data }) {
          ... on Statement {
            id
            orderKey
            revision
            records {
              id
              orderKey
              data
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  const { mutate: deleteRecordMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {
        deleteStatementRecord(input: { id: $id, statementId: $statementId }) {
          ... on Statement {
            id
            orderKey
            revision
            records {
              id
              orderKey
              data
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function createRecord(id: string, statementId: string, orderKey: string, data: JSON) {
    await operations.perform({
      type: "statement.createRecord",
      do: async () => {
        await createRecordMut({
          id: id,
          statementId: statementId,
          orderKey: orderKey,
          data: data,
        });
      },
      undo: async () => {
        await deleteRecordMut({ id: id, statementId: statementId });
      },
    });
  }

  async function updateRecord(id: string, statementId: string, oldData: JSON, newData: JSON) {
    await operations.perform({
      type: "statement.updateRecord",
      do: async () => {
        await updateRecordMut({
          id: id,
          statementId: statementId,
          data: newData,
        });
      },
      undo: async () => {
        await updateRecordMut({
          id: id,
          statementId: statementId,
          data: oldData,
        });
      },
    });
  }

  async function deleteRecord(id: string, statementId: string, orderKey: string, data: JSON) {
    await operations.perform({
      type: "statement.deleteRecord",
      do: async () => {
        await deleteRecordMut({
          id: id,
          statementId: statementId,
        });
      },
      undo: async () => {
        await createRecordMut({
          id,
          statementId,
          orderKey,
          data,
        });
      },
    });
  }

  return { updateStatementDescription, updateStatementCode, createRecord, updateRecord, deleteRecord };
}
