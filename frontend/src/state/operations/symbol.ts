import { graphql } from "@/gql";
import {
  type CreateRecordMutation,
  type DeleteRecordMutation,
  type UpdateRecordMutation,
  type UpdateStatementCodeMutation,
  type UpdateStatementDescriptionMutation,
  type UpdateStatementTextMutation,
  type RestoreRecordMutation,
  type SoftDeleteRecordMutation,
  ModuleMutationType,
} from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
import { OpRegistry, PENDING_REVISION } from "@/state/sync";

export function useSymbolContentOps() {
  const operations = useOperationsStore();
  const registry = new OpRegistry();

  // symbol content mutations

  const { mutate: updateStatementDescriptionMut } = registry.useMutation(
    ModuleMutationType.UpdateStatementDescription,
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

  const { mutate: updateStatementCodeMut } = registry.useMutation(
    ModuleMutationType.UpdateStatementCode,
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

  const { mutate: updateStatementTextMut } = registry.useMutation(
    ModuleMutationType.UpdateStatementText,
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

  const { mutate: createRecordMut } = registry.useMutation(
    ModuleMutationType.CreateRecord,
    graphql(/* GraphQL */ `
      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {
        createRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {
          ... on DatasetRecord {
            id
            createdAt
            updatedAt
            deletedAt
            revision
            orderKey
            data
            statement {
              id
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; statementId: string; orderKey: string; data: any }) =>
        ({
          __typename: "Mutation",
          createRecord: {
            __typename: "DatasetRecord",
            id: vars.id,
            statement: {
              __typename: "Statement",
              id: vars.statementId,
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            revision: PENDING_REVISION,
            orderKey: vars.orderKey,
            data: vars.data,
          },
        } as CreateRecordMutation),
      update(cache, { data: createStatementRecord }) {
        if (createStatementRecord?.createRecord.__typename != "DatasetRecord") {
          return; // error
        }
        // extend Statement.records with the new record
        const newEdge = {
          __typename: "DatasetRecordEdge",
          cursor: btoa(`arrayconnection:0`),
          node: { __ref: cache.identify(createStatementRecord?.createRecord) },
        };
        cache.modify({
          id: cache.identify(createStatementRecord?.createRecord.statement),
          fields: {
            records(existingRecords = { totalCount: 0, edges: [] }) {
              const alreadyExists = existingRecords.edges.some((e: any) => e.node.__ref == newEdge.node.__ref);
              if (alreadyExists) {
                // ignore if already exists
                return existingRecords;
              } else {
                return {
                  totalCount: existingRecords.totalCount + 1,
                  edges: [...existingRecords.edges, newEdge],
                };
              }
            },
          },
          optimistic: true,
        });
      },
    }
  );

  const { mutate: updateRecordMut } = registry.useMutation(
    ModuleMutationType.UpdateRecord,
    graphql(/* GraphQL */ `
      mutation updateRecord($id: GlobalID!, $data: JSON!) {
        updateRecord(input: { id: $id, data: $data }) {
          ... on DatasetRecord {
            id
            updatedAt
            revision
            data
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; data: any }) =>
        ({
          updateRecord: {
            __typename: "DatasetRecord",
            id: vars.id,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            data: vars.data,
          },
        } as UpdateRecordMutation),
    }
  );

  const { mutate: deleteRecordMut } = registry.useMutation(
    ModuleMutationType.DeleteRecord,
    graphql(/* GraphQL */ `
      mutation deleteRecord($id: GlobalID!) {
        deleteRecord(input: { id: $id }) {
          ... on DatasetRecord {
            id
            deletedAt
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string }) =>
        ({
          __typename: "Mutation",
          deleteRecord: {
            __typename: "DatasetRecord",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as DeleteRecordMutation),
    }
  );

  const { mutate: softDeleteRecordMut } = registry.useMutation(
    ModuleMutationType.SoftDeleteRecord,
    graphql(/* GraphQL */ `
      mutation softDeleteRecord($id: GlobalID!) {
        softDeleteRecord(input: { id: $id }) {
          ... on DatasetRecord {
            id
            deletedAt
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string }) =>
        ({
          __typename: "Mutation",
          softDeleteRecord: {
            __typename: "DatasetRecord",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as SoftDeleteRecordMutation),
    }
  );

  const { mutate: restoreRecordMut } = registry.useMutation(
    ModuleMutationType.RestoreRecord,
    graphql(/* GraphQL */ `
      mutation restoreRecord($id: GlobalID!) {
        restoreRecord(input: { id: $id }) {
          ... on DatasetRecord {
            id
            deletedAt
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string }) =>
        ({
          __typename: "Mutation",
          restoreRecord: {
            __typename: "DatasetRecord",
            id: vars.id,
            deletedAt: null,
          },
        } as RestoreRecordMutation),
    }
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
        return await softDeleteRecordMut({ id: id });
      },
      redo: async () => {
        return await restoreRecordMut({ id: id });
      },
    });
  }

  async function updateRecord(id: string, oldData: JSON, newData: JSON) {
    await operations.perform({
      type: "statement.updateRecord",
      do: async () => {
        return await updateRecordMut({ id, data: newData });
      },
      undo: async () => {
        return await updateRecordMut({ id, data: oldData });
      },
    });
  }

  async function deleteRecord(id: string) {
    await operations.perform({
      type: "statement.deleteRecord",
      do: async () => {
        return await deleteRecordMut({ id });
      },
    });
  }

  async function softDeleteRecord(id: string) {
    await operations.perform({
      type: "statement.softDeleteRecord",
      do: async () => {
        return await softDeleteRecordMut({ id });
      },
      undo: async () => {
        return await restoreRecordMut({ id });
      },
    });
  }

  return {
    registry,
    updateStatementDescription,
    updateStatementCode,
    updateStatementText,
    createRecord,
    updateRecord,
    deleteRecord,
    softDeleteRecord,
  };
}
