import { graphql } from "@/gql";
import type {
  UpdateStatementCodeMutation,
  UpdateStatementDescriptionMutation,
  UpdateStatementTextMutation,
} from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";
const PENDING_REVISION = -1;

export function useSymbolContentOps() {
  const operations = useOperationsStore();

  // symbol content mutations

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
    `),
    {
      optimisticResponse: (vars: { id: string; description: string }) =>
        ({
          updateStatementDescription: {
            __typename: "Statement",
            id: vars.id,
            description: vars.description,
            revision: PENDING_REVISION,
          },
        } as UpdateStatementDescriptionMutation),
    }
  );

  async function updateStatementDescription(id: string, oldDescription: string, newDescription: string) {
    await operations.perform({
      type: "statement.updateDescription",
      do: async () => {
        return await updateStatementDescriptionMut({ id: id, description: newDescription });
      },
      undo: async () => {
        return await updateStatementDescriptionMut({ id: id, description: oldDescription });
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
    `),
    {
      optimisticResponse: (vars: { id: string; code: string }) =>
        ({
          updateStatementCode: {
            __typename: "Statement",
            id: vars.id,
            code: vars.code,
            revision: PENDING_REVISION,
          },
        } as UpdateStatementCodeMutation),
    }
  );

  async function updateStatementCode(id: string, oldCode: string, newCode: string) {
    await operations.perform({
      type: "statement.updateCode",
      do: async () => {
        return await updateStatementCodeMut({ id, code: newCode });
      },
      undo: async () => {
        return await updateStatementCodeMut({ id, code: oldCode });
      },
    });
  }

  // text mutation (exactly like code due to reuse but different op) :StatementCodeTextReuse

  const { mutate: updateStatementTextMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateStatementText($id: GlobalID!, $code: String) {
        updateStatementText(input: { id: $id, code: $code }) {
          ... on Statement {
            id
            code
            revision
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; code: string }) =>
        ({
          updateStatementText: {
            __typename: "Statement",
            id: vars.id,
            code: vars.code,
            revision: PENDING_REVISION,
          },
        } as UpdateStatementTextMutation),
    }
  );

  async function updateStatementText(id: string, oldCode: string, newCode: string) {
    await operations.perform({
      type: "statement.updateText",
      do: async () => {
        return await updateStatementTextMut({ id, code: newCode });
      },
      undo: async () => {
        return await updateStatementTextMut({ id, code: oldCode });
      },
    });
  }

  // record mutations

  // TODO @Performance: mutate records optimistically :SubSymbolRevisions
  const { mutate: createRecordMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {
        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {
          ... on Statement {
            id
            orderKey
            revision
            records {
              totalCount
              edges {
                node {
                  id
                  orderKey
                  data
                }
              }
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
              totalCount
              edges {
                node {
                  id
                  orderKey
                  data
                }
              }
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
              totalCount
              edges {
                node {
                  id
                  orderKey
                  data
                }
              }
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
        return await createRecordMut({
          id: id,
          statementId: statementId,
          orderKey: orderKey,
          data: data,
        });
      },
      undo: async () => {
        return await deleteRecordMut({ id: id, statementId: statementId });
      },
    });
  }

  async function updateRecord(id: string, statementId: string, oldData: JSON, newData: JSON) {
    await operations.perform({
      type: "statement.updateRecord",
      do: async () => {
        return await updateRecordMut({
          id: id,
          statementId: statementId,
          data: newData,
        });
      },
      undo: async () => {
        return await updateRecordMut({
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
        return await deleteRecordMut({
          id: id,
          statementId: statementId,
        });
      },
      undo: async () => {
        return await createRecordMut({
          id,
          statementId,
          orderKey,
          data,
        });
      },
    });
  }

  return {
    updateStatementDescription,
    updateStatementCode,
    updateStatementText,
    createRecord,
    updateRecord,
    deleteRecord,
  };
}
