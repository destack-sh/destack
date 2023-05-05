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
  type BatchSoftDeleteRecordMutation,
  type BatchRestoreRecordMutation,
  type RestoreTypeNodeMutation,
  type TypeNodeCreateInput,
  type TypeNodeUpdateInput,
  TypeTag,
  type DeleteTypeNodeMutation,
  type SoftDeleteTypeNodeMutation,
  type UpdateTypeNodeMutation,
} from "@/gql/graphql";
import { useOperationsStore, type Transaction } from "@/state/operations";
import { OpRegistry, PENDING_REVISION } from "@/state/sync";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolContentOps() {
  const ops = useOperationsStore();
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

  async function updateStatementDescription(
    tx: Transaction | null,
    id: string,
    oldDescription: string,
    newDescription: string
  ) {
    await ops.perform({
      tx,
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

  async function updateStatementCode(tx: Transaction | null, id: string, oldCode: string, newCode: string) {
    await ops.perform({
      tx,
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

  async function updateStatementText(tx: Transaction | null, id: string, oldCode: string, newCode: string) {
    await ops.perform({
      tx,
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

  const { mutate: batchSoftDeleteRecordMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation batchSoftDeleteRecord($ids: [GlobalID!]!) {
        batchSoftDeleteRecord(input: { ids: $ids }) {
          ... on RecordBatch {
            records {
              id
              deletedAt
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { ids: string[] }) =>
        ({
          batchSoftDeleteRecord: {
            __typename: "RecordBatch",
            records: vars.ids.map((id) => ({
              __typename: "DatasetRecord",
              id: id,
              deletedAt: new Date().toISOString(),
            })),
          },
        } as BatchSoftDeleteRecordMutation),
    }
  );

  const { mutate: batchRestoreRecordMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation batchRestoreRecord($ids: [GlobalID!]!) {
        batchRestoreRecord(input: { ids: $ids }) {
          ... on RecordBatch {
            records {
              id
              deletedAt
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { ids: string[] }) =>
        ({
          batchRestoreRecord: {
            __typename: "RecordBatch",
            records: vars.ids.map((id) => ({
              __typename: "DatasetRecord",
              id: id,
              deletedAt: null,
            })),
          },
        } as BatchRestoreRecordMutation),
    }
  );

  async function createRecord(tx: Transaction | null, id: string, statementId: string, orderKey: string, data: JSON) {
    await ops.perform({
      tx,
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

  async function updateRecord(tx: Transaction | null, id: string, oldData: JSON, newData: JSON) {
    await ops.perform({
      tx,
      type: "statement.updateRecord",
      do: async () => {
        return await updateRecordMut({ id, data: newData });
      },
      undo: async () => {
        return await updateRecordMut({ id, data: oldData });
      },
    });
  }

  async function deleteRecord(tx: Transaction | null, id: string) {
    await ops.perform({
      tx,
      type: "statement.deleteRecord",
      do: async () => {
        return await deleteRecordMut({ id });
      },
    });
  }

  async function softDeleteRecord(tx: Transaction | null, id: string) {
    await ops.perform({
      tx,
      type: "statement.softDeleteRecord",
      do: async () => {
        return await softDeleteRecordMut({ id });
      },
      undo: async () => {
        return await restoreRecordMut({ id });
      },
    });
  }

  async function batchSoftDeleteRecord(ids: string[]) {
    await ops.perform({
      type: "statement.batchSoftDeleteRecord",
      do: async () => {
        return await batchSoftDeleteRecordMut({ ids });
      },
      undo: async () => {
        return await batchRestoreRecordMut({ ids });
      },
    });
  }

  async function batchRestoreRecord(ids: string[]) {
    await ops.perform({
      type: "statement.batchRestoreRecord",
      do: async () => {
        return await batchRestoreRecordMut({ ids });
      },
      undo: async () => {
        return await batchSoftDeleteRecordMut({ ids });
      },
    });
  }

  const { mutate: createTypeNodeMut } = registry.useMutation(
    ModuleMutationType.CreateTypeNode,
    graphql(/* GraphQL */ `
      mutation createTypeNode(
        $id: GlobalID!
        $statementId: GlobalID!
        $tag: TypeTag!
        $orderKey: String!
        $name: String!
        $description: String
        $isOutput: Boolean!
        $isArray: Boolean!
        $isNullable: Boolean!
        $value: JSON
        $referenceId: GlobalID
      ) {
        createTypeNode(
          input: {
            id: $id
            statementId: $statementId
            tag: $tag
            orderKey: $orderKey
            name: $name
            description: $description
            isOutput: $isOutput
            isArray: $isArray
            isNullable: $isNullable
            value: $value
            referenceId: $referenceId
          }
        ) {
          ... on SimpleTypeNode {
            # should match SimpleTypeNodeContent fragment
            id
            createdAt
            updatedAt
            deletedAt
            orderKey
            statement {
              id
            }
            revision
            name
            tag
            description
            value
            reference {
              id
            }
            isOutput
            isArray
            isNullable
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        id: string;
        tag: string;
        orderKey: string;
        statementId: string;
        name: string;
        description: string | null;
        isOutput: boolean;
        isArray: boolean;
        isNullable: boolean;
        value: any;
        referenceId: string | null;
      }) =>
        ({
          __typename: "Mutation",
          createTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.id,
            statement: {
              __typename: "Statement",
              id: vars.statementId,
            },
            revision: PENDING_REVISION,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            tag: vars.tag,
            name: vars.name,
            description: vars.description ?? null,
            value: vars.value,
            orderKey: vars.orderKey,
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
            isOutput: vars.isOutput,
            isArray: vars.isArray,
            isNullable: vars.isNullable,
          },
        } as any),
      update(cache, { data }) {
        const createStatementTypeNode = data?.createTypeNode;
        if (createStatementTypeNode?.__typename != "SimpleTypeNode") {
          return; // error
        }
        // extend Statement.type_nodes with (ref to) new type node
        cache.modify({
          id: cache.identify(createStatementTypeNode.statement),
          fields: {
            typeNodes(existingTypeNodes = []) {
              const newRef = cache.identify(createStatementTypeNode);
              return [
                ...existingTypeNodes.filter((t: any) => t.__ref != newRef), // remove old type node if exists
                { __ref: newRef },
              ];
            },
          },
          optimistic: true,
        });
      },
    }
  );

  const { mutate: deleteTypeNodeMut } = registry.useMutation(
    ModuleMutationType.DeleteTypeNode,
    graphql(/* GraphQL */ `
      mutation deleteTypeNode($id: GlobalID!) {
        deleteTypeNode(input: { id: $id }) {
          ... on SimpleTypeNode {
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
          deleteTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as DeleteTypeNodeMutation),
    }
  );

  const { mutate: softDeleteTypeNodeMut } = registry.useMutation(
    ModuleMutationType.SoftDeleteTypeNode,
    graphql(/* GraphQL */ `
      mutation softDeleteTypeNode($id: GlobalID!) {
        softDeleteTypeNode(input: { id: $id }) {
          ... on SimpleTypeNode {
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
          softDeleteTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as SoftDeleteTypeNodeMutation),
    }
  );

  const { mutate: restoreTypeNodeMut } = registry.useMutation(
    ModuleMutationType.RestoreTypeNode,
    graphql(/* GraphQL */ `
      mutation restoreTypeNode($id: GlobalID!) {
        restoreStatementTypeNode(input: { id: $id }) {
          ... on SimpleTypeNode {
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
          restoreStatementTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.id,
            deletedAt: null,
          },
        } as RestoreTypeNodeMutation),
    }
  );

  function _toTypeNodeInput(input: TypeNodeCreateInput) {
    return {
      ...input,
      // set optional values to null if not provided
      description: input.description ?? null,
      value: input.value ?? null,
      referenceId: input.referenceId ?? null,
      isArray: input.isArray ?? false,
      isNullable: input.isNullable ?? false,
      isOutput: input.isOutput ?? false,
    } as TypeNodeCreateInput;
  }

  async function createTypeNode(tx: Transaction | null, statementId: string, typeNode: TypeNodeCreateInput) {
    await ops.perform({
      tx,
      type: "symbol.createTypeNode",
      do: async () => {
        return await createTypeNodeMut(_toTypeNodeInput(typeNode));
      },
      undo: async () => {
        return await softDeleteTypeNodeMut({ id: typeNode.id });
      },
      redo: async () => {
        return await restoreTypeNodeMut({ id: typeNode.id });
      },
    });
  }

  async function deleteTypeNode(tx: Transaction | null, statementId: string, typeNode: TypeNodeCreateInput) {
    await ops.perform({
      tx,
      type: "symbol.deleteTypeNode",
      do: async () => {
        return await deleteTypeNodeMut({ id: typeNode.id });
      },
    });
  }

  async function softDeleteTypeNode(tx: Transaction | null, statementId: string, typeNode: TypeNodeCreateInput) {
    await ops.perform({
      tx,
      type: "symbol.softDeleteTypeNode",
      do: async () => {
        return await softDeleteTypeNodeMut({ id: typeNode.id });
      },
      undo: async () => {
        return await restoreTypeNodeMut({ id: typeNode.id });
      },
    });
  }

  const { mutate: updateTypeNodeMut } = registry.useMutation(
    ModuleMutationType.UpdateTypeNode,
    graphql(/* GraphQL */ `
      mutation updateTypeNode(
        $id: GlobalID!
        $tag: TypeTag!
        $name: String
        $description: String
        $isOutput: Boolean!
        $isArray: Boolean!
        $isNullable: Boolean!
        $value: JSON
        $referenceId: GlobalID
      ) {
        updateTypeNode(
          input: {
            id: $id
            tag: $tag
            name: $name
            description: $description
            isOutput: $isOutput
            isArray: $isArray
            isNullable: $isNullable
            value: $value
            referenceId: $referenceId
          }
        ) {
          ... on SimpleTypeNode {
            id
            tag
            updatedAt
            revision
            name
            description
            isOutput
            isArray
            isNullable
            value
            reference {
              id
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        id: string;
        tag: TypeTag;
        name: string | null;
        description: string;
        isOutput: boolean;
        isArray: boolean;
        isNullable: boolean;
        value: any;
        referenceId?: string;
      }) => {
        return {
          updateTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.id,
            tag: vars.tag,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            name: vars.name,
            description: vars.description,
            isOutput: vars.isOutput,
            isArray: vars.isArray,
            isNullable: vars.isNullable,
            value: vars.value,
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
          },
        } as UpdateTypeNodeMutation;
      },
    }
  );

  async function updateTypeNode(
    tx: Transaction | null,
    oldTypeNode: TypeNodeUpdateInput,
    newTypeNode: TypeNodeUpdateInput
  ) {
    await ops.perform({
      tx,
      type: "symbol.updateTypeNode",
      do: async () => {
        return await updateTypeNodeMut(newTypeNode);
      },
      undo: async () => {
        return await updateTypeNodeMut(oldTypeNode);
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
    batchSoftDeleteRecord,
    batchRestoreRecord,
    createTypeNode,
    updateTypeNode,
    deleteTypeNode,
    softDeleteTypeNode,
  };
}
