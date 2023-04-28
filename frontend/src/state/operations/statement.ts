import { graphql } from "@/gql";
import {
  StatementType,
  TypeTag,
  type BatchDeleteStatementsMutation,
  type BatchMoveStatementMutation,
  type BatchRestoreStatementsMutation,
  type CreateTypeNodeMutation,
  type DeleteStatementMutation,
  type DeleteTypeNodeMutation,
  type MorphStatementMutation,
  type MoveStatementMutation,
  type RenameStatementMutation,
  type RestoreStatementMutation,
  type RestoreTypeNodeMutation,
  type StatementModifier,
  type StatementMorphInput,
  type SymbolType,
  type TypeNodeCreateInput,
  type TypeNodeUpdateInput,
  type UpdateStatementModifierMutation,
  type UpdateTypeNodeMutation,
  type SetReferenceMutation,
  type CommentStatementMutation,
  type CreateStatementBlankMutation,
  type SoftDeleteStatementMutation,
  type SoftDeleteTypeNodeMutation,
  ModuleMutationType,
} from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
import { OpRegistry, PENDING_REVISION } from "@/state/sync";
import { useMutation } from "@vue/apollo-composable";
import { v4 as uuidv4 } from "uuid";

export function newStatementId(): string {
  /* Generates a new statement global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`Statement:${nodeId}`);
}

export function newTypeNodeId(): string {
  /* Generates a new type node data global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`SimpleTypeNode:${nodeId}`);
}

export function newDatasetRecordId(): string {
  /* Generates a new dataset record global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`DatasetRecord:${nodeId}`);
}

export function useStatementOps() {
  const operations = useOperationsStore();
  const registry = new OpRegistry();

  const { mutate: createStatementBlankMut } = registry.useMutation(
    ModuleMutationType.CreateStatementBlank,
    graphql(/* GraphQL */ `
      mutation createStatementBlank($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {
        createStatementBlank(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {
          ... on Statement {
            # should match STatementContent fragment
            id
            type
            revision
            symbolType
            createdAt
            updatedAt
            deletedAt
            name
            commented
            generated
            modifier
            orderKey
            file {
              id
            }
            parent {
              id
            }
            reference {
              id
            }
            # symbol contents
            lang
            code
            description
            referenceProjectVersion {
              id
            }
            rootTypeTag
            typeNodes(filters: { isVisible: true }) {
              id
            }
            records(filters: { isVisible: true }) {
              totalCount
              edges {
                node {
                  id
                }
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; fileId: string; parentId: string | null; orderKey: string }) =>
        ({
          __typename: "Mutation",
          createStatementBlank: {
            __typename: "Statement",
            id: vars.id,
            file: {
              __typename: "File",
              id: vars.fileId,
            },
            parent: vars.parentId == null ? null : { __typename: "Statement", id: vars.parentId },
            revision: PENDING_REVISION,
            orderKey: vars.orderKey,
            // default new fields (all! fields in StatementContent fragment)
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            type: StatementType.Blank,
            modifier: null,
            name: null,
            symbolType: null,
            description: null,
            value: null,
            code: null,
            referenceProjectVersion: null,
            records: {
              totalCount: 0,
              edges: [],
            },
            rootTypeTag: null,
            typeNodes: [],
            lang: null,
            reference: null,
            generated: false,
            commented: false,
          },
        } as CreateStatementBlankMutation),
      update(cache, { data }) {
        if (data?.createStatementBlank.__typename != "Statement") {
          return; // error
        }
        // extend File.statements array with (ref to) new statement
        // must ensure that all relevant fields are present or weird things happen
        cache.modify({
          id: cache.identify(data.createStatementBlank?.file),
          fields: {
            statements(currentStatements = []) {
              return [...currentStatements, { __ref: cache.identify(data?.createStatementBlank) }];
            },
          },
          optimistic: true,
        });
      },
    }
  );

  async function createBlank(id: string, fileId: string, parentId: string | null, orderKey: string) {
    async function apply() {
      return await createStatementBlankMut({
        id,
        fileId,
        parentId,
        orderKey,
      });
    }

    return await operations.perform({
      type: "statement.create",
      do: apply,
      undo: async () => {
        await softDeleteStatementMut({ id });
      },
      redo: async () => {
        return await restoreStatementMut({ id });
      },
    });
  }

  const { mutate: morphStatementMut } = registry.useMutation(
    ModuleMutationType.MorphStatement,
    graphql(/* GraphQL */ `
      mutation morphStatement($input: StatementMorphInput!) {
        morphStatement(input: $input) {
          ... on Statement {
            id
            revision
            type
            symbolType
            name
            rootTypeTag
            lang
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { input: StatementMorphInput }) =>
        ({
          morphStatement: {
            __typename: "Statement",
            id: vars.input.id,
            revision: PENDING_REVISION,
            type: vars.input.type,
            symbolType: vars.input.symbolType ?? null,
            name: vars.input.name ?? null,
            rootTypeTag: vars.input.rootTypeTag ?? null,
            lang: vars.input.lang ?? null,
          },
        } as MorphStatementMutation),
    }
  );

  async function morph(
    id: string,
    oldStatement: {
      type: StatementType;
      symbolType?: SymbolType;
      name?: string;
      rootTypeTag?: TypeTag;
      lang?: string;
    },
    newStatement: {
      type: StatementType;
      symbolType?: SymbolType;
      name?: string;
      rootTypeTag?: TypeTag;
      lang?: string;
    }
  ) {
    await operations.perform({
      type: "statement.morph",
      do: async () => {
        return await morphStatementMut({ input: { id, ...newStatement } });
      },
      undo: async () => {
        return await morphStatementMut({ input: { id, ...oldStatement } });
      },
    });
  }

  const { mutate: updateStatementModifier } = registry.useMutation(
    ModuleMutationType.UpdateStatementModifier,
    graphql(/* GraphQL */ `
      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {
        updateStatementModifier(input: { id: $id, modifier: $modifier }) {
          ... on Statement {
            id
            modifier
            revision
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; modifier: StatementModifier | null }) =>
        ({
          updateStatementModifier: {
            __typename: "Statement",
            id: vars.id,
            modifier: vars.modifier,
            revision: PENDING_REVISION,
          },
        } as UpdateStatementModifierMutation),
    }
  );

  async function modify(id: string, oldModifier: StatementModifier | null, newModifier: StatementModifier | null) {
    await operations.perform({
      type: "statement.modify",
      do: async () => {
        return await updateStatementModifier({ id: id, modifier: newModifier });
      },
      undo: async () => {
        return await updateStatementModifier({ id: id, modifier: oldModifier });
      },
    });
  }

  const { mutate: moveStatementMut } = registry.useMutation(
    ModuleMutationType.MoveStatement,
    graphql(/* GraphQL */ `
      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {
        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {
          ... on Statement {
            id
            orderKey
            revision
            file {
              id
            }
            parent {
              id
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; fileId: string; parentId?: string; orderKey: string }) =>
        ({
          moveStatement: {
            __typename: "Statement",
            id: vars.id,
            orderKey: vars.orderKey,
            file: {
              id: vars.fileId,
            },
            revision: PENDING_REVISION,
            parent: vars.parentId ? { id: vars.parentId } : null,
          },
        } as MoveStatementMutation),
    }
  );

  const { mutate: batchMoveStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation batchMoveStatement(
        $ids: [GlobalID!]!
        $fileId: GlobalID!
        $parentIds: [GlobalID]!
        $orderKeys: [String!]!
      ) {
        batchMoveStatement(input: { ids: $ids, fileId: $fileId, parentIds: $parentIds, orderKeys: $orderKeys }) {
          ... on StatementBatch {
            statements {
              id
              orderKey
              revision
              file {
                id
              }
              parent {
                id
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        ids: string[];
        fileId: string;
        parentIds: (string | undefined)[];
        orderKeys: string[];
      }) =>
        ({
          batchMoveStatement: {
            __typename: "StatementBatch",
            statements: vars.ids.map((id, i) => ({
              __typename: "Statement",
              id: id,
              orderKey: vars.orderKeys[i],
              file: {
                id: vars.fileId,
              },
              revision: PENDING_REVISION,
              parent: vars.parentIds[i] ? { id: vars.parentIds[i] } : null,
            })),
          },
        } as BatchMoveStatementMutation),
    }
  );

  async function move(
    id: string,
    oldLoc: { fileId: string; parentId?: string; orderKey: string },
    newLoc: { fileId: string; parentId?: string; orderKey: string }
  ) {
    await operations.perform({
      type: "statement.move",
      do: async () => {
        return await moveStatementMut({
          id: id,
          fileId: newLoc.fileId,
          parentId: newLoc.parentId,
          orderKey: newLoc.orderKey,
        });
      },
      undo: async () => {
        return await moveStatementMut({
          id: id,
          fileId: oldLoc.fileId,
          parentId: oldLoc.parentId,
          orderKey: oldLoc.orderKey,
        });
      },
    });
  }

  async function batchMove(
    ids: string[],
    oldLocs: { fileId: string; parentId?: string; orderKey: string }[],
    newLocs: { fileId: string; parentId?: string; orderKey: string }[]
  ) {
    await operations.perform({
      type: "statement.batchMove",
      do: async () => {
        return await batchMoveStatementMut({
          ids: ids,
          fileId: newLocs[0].fileId,
          parentIds: newLocs.map((loc) => loc.parentId),
          orderKeys: newLocs.map((loc) => loc.orderKey),
        });
      },
      undo: async () => {
        return await batchMoveStatementMut({
          ids: ids,
          fileId: oldLocs[0].fileId,
          parentIds: oldLocs.map((loc) => loc.parentId),
          orderKeys: oldLocs.map((loc) => loc.orderKey),
        });
      },
    });
  }

  const { mutate: renameStatementMut } = registry.useMutation(
    ModuleMutationType.RenameStatement,
    graphql(/* GraphQL */ `
      mutation renameStatement($id: GlobalID!, $name: String) {
        renameStatement(input: { id: $id, name: $name }) {
          ... on Statement {
            id
            name
            revision
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; name: string | null }) =>
        ({
          renameStatement: {
            __typename: "Statement",
            id: vars.id,
            name: vars.name,
            revision: PENDING_REVISION,
          },
        } as RenameStatementMutation),
    }
  );

  async function rename(id: string, oldName: string | null, newName: string | null) {
    await operations.perform({
      type: "statement.rename",
      do: async () => {
        return await renameStatementMut({ id: id, name: newName });
      },
      undo: async () => {
        return await renameStatementMut({ id: id, name: oldName });
      },
    });
  }

  const { mutate: deleteStatementMut } = registry.useMutation(
    ModuleMutationType.DeleteStatement,
    // we don't bother updating descendants here since they will be automatically hidden
    // when their parent/ancestor is deleted (and its more responsive that way on restore)
    graphql(/* GraphQL */ `
      mutation deleteStatement($id: GlobalID!) {
        deleteStatement(input: { id: $id }) {
          ... on Statement {
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
          deleteStatement: {
            __typename: "Statement",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as DeleteStatementMutation),
    }
  );

  const { mutate: softDeleteStatementMut } = registry.useMutation(
    ModuleMutationType.SoftDeleteStatement,
    // we don't bother updating descendants here since they will be automatically hidden
    // when their parent/ancestor is deleted (and its more responsive that way on restore)
    graphql(/* GraphQL */ `
      mutation softDeleteStatement($id: GlobalID!) {
        softDeleteStatement(input: { id: $id }) {
          ... on Statement {
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
          softDeleteStatement: {
            __typename: "Statement",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as SoftDeleteStatementMutation),
    }
  );

  const { mutate: batchSoftDeleteStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation batchDeleteStatements($ids: [GlobalID!]!) {
        batchSoftDeleteStatement(input: { ids: $ids }) {
          ... on StatementBatch {
            statements {
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
          batchSoftDeleteStatement: {
            __typename: "StatementBatch",
            statements: vars.ids.map((id) => ({
              __typename: "Statement",
              id: id,
              deletedAt: new Date().toISOString(),
            })),
          },
        } as BatchDeleteStatementsMutation),
    }
  );

  const { mutate: restoreStatementMut } = registry.useMutation(
    ModuleMutationType.RestoreStatement,
    graphql(/* GraphQL */ `
      mutation restoreStatement($id: GlobalID!) {
        restoreStatement(input: { id: $id }) {
          ... on Statement {
            id
            deletedAt
            descendants {
              id
              deletedAt
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string }) =>
        ({
          restoreStatement: {
            __typename: "Statement",
            id: vars.id,
            deletedAt: null,
            descendants: [], // unknown
          },
        } as RestoreStatementMutation),
    }
  );

  const { mutate: batchRestoreStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation batchRestoreStatements($ids: [GlobalID!]!) {
        batchRestoreStatement(input: { ids: $ids }) {
          ... on StatementBatch {
            statements {
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
          batchRestoreStatement: {
            __typename: "StatementBatch",
            statements: vars.ids.map((id) => ({
              __typename: "Statement",
              id: id,
              deletedAt: null,
            })),
          },
        } as BatchRestoreStatementsMutation),
    }
  );

  async function delete_(id: string) {
    await operations.perform({
      type: "statement.delete",
      do: async () => {
        return await deleteStatementMut({ id: id });
      },
    });
  }

  async function softDelete(id: string) {
    await operations.perform({
      type: "statement.softDelete",
      do: async () => {
        return await softDeleteStatementMut({ id: id });
      },
      undo: async () => {
        return await restoreStatementMut({ id: id });
      },
    });
  }

  async function batchSoftDelete_(ids: string[]) {
    await operations.perform({
      type: "statement.batchDelete",
      do: async () => {
        return await batchSoftDeleteStatementMut({ ids: ids });
      },
      undo: async () => {
        return await batchRestoreStatementMut({ ids: ids });
      },
    });
  }

  // TODO @Performance: make statement paste optimistic
  const { mutate: batchPasteMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation batchPasteStatement(
        $sourceIds: [GlobalID!]!
        $targetIds: [GlobalID!]!
        $targetFileId: GlobalID!
        $targetParentIds: [GlobalID]!
        $targetOrderKeys: [String!]!
      ) {
        batchPasteStatement(
          input: {
            sourceIds: $sourceIds
            targetIds: $targetIds
            targetFileId: $targetFileId
            targetParentIds: $targetParentIds
            targetOrderKeys: $targetOrderKeys
          }
        ) {
          ... on StatementBatch {
            statements {
              id
              ...StatementContent
              file {
                id
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      update(cache, { data: batchPasteStatement }) {
        if (batchPasteStatement?.batchPasteStatement.__typename != "StatementBatch") {
          return; // error
        }
        // extend File.statements array with (ref to) new statements
        batchPasteStatement.batchPasteStatement.statements.forEach((statement) => {
          cache.modify({
            id: cache.identify(statement.file),
            fields: {
              statements(currentStatements = []) {
                return [...currentStatements, { __ref: cache.identify(statement) }];
              },
            },
            optimistic: true,
          });
        });
      },
    }
  );

  async function batchPaste(
    sourceIds: string[],
    targetIds: string[],
    targetFileId: string,
    targetParentIds: (string | null)[],
    targetOrderKeys: string[]
  ) {
    await operations.perform({
      type: "statement.batchPaste",
      do: async () => {
        return await batchPasteMut({
          sourceIds: sourceIds,
          targetIds: targetIds,
          targetFileId: targetFileId,
          targetParentIds: targetParentIds,
          targetOrderKeys: targetOrderKeys,
        });
      },
      undo: async () => {
        return await batchSoftDeleteStatementMut({ ids: targetIds });
      },
      redo: async () => {
        return await batchRestoreStatementMut({ ids: targetIds });
      },
    });
  }

  const { mutate: commentStatementMut } = registry.useMutation(
    ModuleMutationType.CommentStatement,
    // TODO @UX: uncommenting nested statements feels janky
    graphql(/* GraphQL */ `
      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {
        commentStatement(input: { id: $id, commented: $commented }) {
          ... on Statement {
            id
            commented
            revision
            descendants {
              id
              commented
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; commented: boolean }) =>
        ({
          commentStatement: {
            __typename: "Statement",
            id: vars.id,
            commented: vars.commented,
            revision: 0,
            descendants: [], // unknown
          },
        } as CommentStatementMutation),
    }
  );

  async function comment(id: string, commented: boolean) {
    await operations.perform({
      type: "statement.comment",
      do: async () => {
        return await commentStatementMut({ id: id, commented: commented });
      },
      undo: async () => {
        return await commentStatementMut({ id: id, commented: !commented });
      },
    });
  }

  const { mutate: setReferenceMut } = registry.useMutation(
    ModuleMutationType.UpdateStatementReference,
    graphql(/* GraphQL */ `
      mutation setReference($id: GlobalID!, $referenceId: GlobalID, $referenceName: String) {
        updateStatementReference(input: { id: $id, referenceId: $referenceId, referenceName: $referenceName }) {
          ... on Statement {
            id
            revision
            reference {
              id
              name
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; referenceId: string | null; referenceName: string | null }) =>
        ({
          updateStatementReference: {
            __typename: "Statement",
            id: vars.id,
            revision: PENDING_REVISION,
            reference: vars.referenceId
              ? {
                  __typename: "Statement",
                  id: vars.referenceId,
                  name: vars.referenceName,
                }
              : null,
          },
        } as SetReferenceMutation),
    }
  );

  async function setReference(
    id: string,
    oldReferenceId: string | null,
    oldReferenceName: string | null,
    newReferenceId: string | null,
    newReferenceName: string | null
  ) {
    await operations.perform({
      type: "statement.setReference",
      do: async () => {
        return await setReferenceMut({ id: id, referenceId: newReferenceId, referenceName: newReferenceName });
      },
      undo: async () => {
        return await setReferenceMut({ id: id, referenceId: oldReferenceId, referenceName: oldReferenceName });
      },
    });
  }

  const { mutate: createTypeNodeMut } = registry.useMutation(
    ModuleMutationType.CreateTypeNode,
    graphql(/* GraphQL */ `
      mutation createTypeNode($typeNode: TypeNodeCreateInput!) {
        createTypeNode(input: $typeNode) {
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
      optimisticResponse: (vars: { typeNode: TypeNodeCreateInput }) =>
        ({
          __typename: "Mutation",
          createTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.typeNode.id,
            statement: {
              __typename: "Statement",
              id: vars.typeNode.statementId,
            },
            revision: PENDING_REVISION,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            tag: vars.typeNode.tag,
            name: vars.typeNode.name,
            description: vars.typeNode.description ?? null,
            value: vars.typeNode.value,
            orderKey: vars.typeNode.orderKey,
            reference:
              vars.typeNode.referenceId == null ? null : { __typename: "Statement", id: vars.typeNode.referenceId },
            isOutput: vars.typeNode.isOutput,
            isArray: vars.typeNode.isArray,
            isNullable: vars.typeNode.isNullable,
          },
        } as CreateTypeNodeMutation),
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
              return [...existingTypeNodes, { __ref: cache.identify(createStatementTypeNode) }];
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

  async function createTypeNode(statementId: string, typeNode: TypeNodeCreateInput) {
    await operations.perform({
      type: "statement.createTypeNode",
      do: async () => {
        return await createTypeNodeMut({ typeNode: typeNode });
      },
      undo: async () => {
        return await softDeleteTypeNodeMut({ id: typeNode.id });
      },
      redo: async () => {
        return await restoreTypeNodeMut({ id: typeNode.id });
      },
    });
  }

  async function deleteTypeNode(statementId: string, typeNode: TypeNodeCreateInput) {
    await operations.perform({
      type: "statement.deleteTypeNode",
      do: async () => {
        return await deleteTypeNodeMut({ id: typeNode.id });
      },
    });
  }

  async function softDeleteTypeNode(statementId: string, typeNode: TypeNodeCreateInput) {
    await operations.perform({
      type: "statement.softDeleteTypeNode",
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
      mutation updateTypeNode($typeNode: TypeNodeUpdateInput!) {
        updateTypeNode(input: $typeNode) {
          ... on SimpleTypeNode {
            id
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
      optimisticResponse: (vars: { typeNode: TypeNodeUpdateInput }) =>
        ({
          updateTypeNode: {
            __typename: "SimpleTypeNode",
            id: vars.typeNode.id,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            name: vars.typeNode.name,
            description: vars.typeNode.description,
            isOutput: vars.typeNode.isOutput,
            isArray: vars.typeNode.isArray,
            isNullable: vars.typeNode.isNullable,
            value: vars.typeNode.value,
            reference:
              vars.typeNode.referenceId == null ? null : { __typename: "Statement", id: vars.typeNode.referenceId },
          },
        } as UpdateTypeNodeMutation),
    }
  );

  async function updateTypeNode(oldTypeNode: TypeNodeUpdateInput, newTypeNode: TypeNodeUpdateInput) {
    await operations.perform({
      type: "statement.updateTypeNode",
      do: async () => {
        return await updateTypeNodeMut({ typeNode: newTypeNode });
      },
      undo: async () => {
        return await updateTypeNodeMut({ typeNode: oldTypeNode });
      },
    });
  }

  return {
    registry,
    create: createBlank,
    morph,
    modify,
    setReference,
    move,
    batchMove,
    batchPaste,
    comment,
    rename,
    delete: delete_,
    softDelete: softDelete,
    batchSoftDelete: batchSoftDelete_,
    createTypeNode,
    updateTypeNode,
    deleteTypeNode,
    softDeleteTypeNode,
  };
}
