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
  type TruncateRecordsMutation,
  type DeleteTypeNodeMutation,
  type SoftDeleteTypeNodeMutation,
  type UpdateTypeNodeMutation,
  type Scalars,
  TypeHint,
} from "@/gql/graphql";
import { useOperationsStore, type Transaction } from "@/state/operations";
import { OpRegistry, PENDING_REVISION } from "@/state/sync";
import { ArrowDownCircleIcon, ArrowDownIcon } from "@heroicons/vue/24/outline";
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
      update(cache, { data }) {
        if (data?.createRecord.__typename != "DatasetRecord") {
          return; // error
        }

        function readOrderKey(recordRef: any): string {
          const record = cache.readFragment({
            id: recordRef,
            fragment: graphql(/* GraphQL */ `
              fragment _orderKey on DatasetRecord {
                orderKey
              }
            `),
          });
          if (record == null) {
            throw new Error("record not in cache: " + recordRef);
          }
          return record.orderKey;
        }

        // extend relevant records views with the new record
        const newEdge = {
          __typename: "DatasetRecordEdge",
          // cursor is set below
          node: { __ref: cache.identify(data?.createRecord) },
        };
        cache.modify({
          id: cache.identify(data?.createRecord.statement),
          fields: {
            //  :StatementRecordsView
            records({ value: existing, args }) {
              if (!existing) {
                existing = {
                  edges: [],
                  totalCount: 0,
                  pageInfo: {
                    hasPreviousPage: false,
                    hasNextPage: false,
                    startCursor: "",
                    endCursor: "",
                  },
                };
              }
              if (existing.edges.some((e: any) => e.node.__ref == newEdge.node.__ref)) {
                return { value: existing, args };
              }

              // retain order key order
              const newEdges = [...existing.edges, newEdge].sort((a: any, b: any) => {
                return readOrderKey(a.node.__ref) < readOrderKey(b.node.__ref) ? -1 : 1;
              });
              // compute cursor based on surrounding edges
              const inPageIndex = newEdges.findIndex((e: any) => e.node.__ref == newEdge.node.__ref);

              // filter out records outside of the current page (if synced from elsewhere)
              if ((data as any).synced && inPageIndex >= args.first) {
                // if synced, don't add the record if it's outside the current page
                const pageInfo = {
                  ...existing.pageInfo,
                  hasNextPage: true,
                };
                return { value: { ...existing, pageInfo }, args };
              }

              let cursorIndex;
              if (inPageIndex > 0) {
                const beforeCursor = newEdges[inPageIndex - 1].cursor;
                const beforeIndex = atob(beforeCursor).split(":")[1];
                cursorIndex = parseInt(beforeIndex) + 1;
              } else if (inPageIndex < newEdges.length - 1) {
                const afterCursor = newEdges[inPageIndex + 1].cursor;
                const afterIndex = atob(afterCursor).split(":")[1];
                cursorIndex = parseInt(afterIndex) - 1;
              } else {
                cursorIndex = 0;
              }
              newEdges[inPageIndex] = {
                ...newEdge,
                cursor: btoa(`arrayconnection:` + cursorIndex), // see strawberry graphql connection internals
              };
              return {
                value: {
                  ...existing,
                  totalCount: -1, // unknown
                  edges: newEdges,
                },
                args,
              };
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

  const { mutate: truncateRecordsMut } = registry.useMutation(
    ModuleMutationType.TruncateRecords,
    graphql(/* GraphQL */ `
      mutation truncateRecords($id: GlobalID!) {
        truncateRecords(input: { id: $id }) {
          ... on Statement {
            id
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string }) =>
        ({
          truncateRecords: {
            __typename: "Statement",
            id: vars.id,
          },
        } as TruncateRecordsMutation),
      update(cache, { data }) {
        // wipe all records from the cache
        // :StatementRecordsView
        cache.modify({
          id: cache.identify(data?.truncateRecords),
          fields: {
            records({ args }) {
              return {
                value: {
                  totalCount: 0,
                  pageInfo: {
                    hasNextPage: false,
                    hasPreviousPage: false,
                    startCursor: null,
                    endCursor: null,
                  },
                  edges: [],
                },
                args,
              };
            },
          },
        });
      },
    }
  );

  async function createRecord(
    tx: Transaction | null,
    id: string,
    statementId: string,
    orderKey: string,
    data: Scalars["JSON"]
  ) {
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

  async function updateRecord(tx: Transaction | null, id: string, oldData: Scalars["JSON"], newData: Scalars["JSON"]) {
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

  async function truncateRecords(statementId: string) {
    await ops.perform({
      type: "statement.truncateRecords",
      do: async () => {
        return await truncateRecordsMut({ id: statementId });
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
        $hint: TypeHint
        $key: String!
        $orderKey: String!
        $name: String!
        $description: String
        $flags: Int!
        $value: JSON
        $referenceId: GlobalID
      ) {
        createTypeNode(
          input: {
            id: $id
            statementId: $statementId
            tag: $tag
            hint: $hint
            key: $key
            orderKey: $orderKey
            name: $name
            description: $description
            flags: $flags
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
            key
            orderKey
            statement {
              id
            }
            revision
            name
            tag
            hint
            description
            value
            reference {
              id
            }
            flags
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        id: string;
        tag: string;
        hint: string | null;
        key: string;
        orderKey: string;
        statementId: string;
        name: string;
        description: string | null;
        flags: number;
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
            hint: vars.hint ?? null,
            name: vars.name,
            key: vars.key,
            description: vars.description ?? null,
            value: vars.value,
            orderKey: vars.orderKey,
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
            flags: vars.flags,
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
      hint: input.hint ?? null,
      description: input.description ?? null,
      value: input.value ?? null,
      referenceId: input.referenceId ?? null,
      flags: input.flags ?? 0,
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
        $hint: TypeHint
        $name: String
        $description: String
        $flags: Int!
        $value: JSON
        $referenceId: GlobalID
      ) {
        updateTypeNode(
          input: {
            id: $id
            tag: $tag
            hint: $hint
            name: $name
            description: $description
            flags: $flags
            value: $value
            referenceId: $referenceId
          }
        ) {
          ... on SimpleTypeNode {
            id
            tag
            hint
            updatedAt
            revision
            name
            description
            flags
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
        hint: TypeHint | null;
        name: string | null;
        description: string;
        flags: number;
        value: any;
        referenceId?: string;
      }) => {
        return {
          updateTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.id,
            tag: vars.tag,
            hint: vars.hint ?? null,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            name: vars.name,
            description: vars.description,
            flags: vars.flags,
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

  const { mutate: moveTypeNodeMut } = registry.useMutation(
    ModuleMutationType.MoveTypeNode,
    graphql(/* GraphQL */ `
      mutation moveTypeNode($id: GlobalID!, $orderKey: String!) {
        moveTypeNode(input: { id: $id, orderKey: $orderKey }) {
          ... on SimpleTypeNode {
            id
            orderKey
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; orderKey: string }) =>
        ({
          moveTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.id,
            orderKey: vars.orderKey,
          },
        } as any),
    }
  );

  async function moveTypeNode(tx: Transaction | null, id: string, oldOrderKey: string, newOrderKey: string) {
    await ops.perform({
      tx,
      type: "symbol.moveTypeNode",
      do: async () => {
        return await moveTypeNodeMut({ id, orderKey: newOrderKey });
      },
      undo: async () => {
        return await moveTypeNodeMut({ id, orderKey: oldOrderKey });
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
    truncateRecords,
    createTypeNode,
    updateTypeNode,
    moveTypeNode,
    deleteTypeNode,
    softDeleteTypeNode,
  };
}
