import { graphql } from "@/gql";
import {
  ModuleMutationType,
  StatementType,
  TypeTag,
  type BatchDeleteStatementsMutation,
  type BatchMoveStatementMutation,
  type BatchRestoreStatementsMutation,
  type CommentStatementMutation,
  type CreateStatementMutation,
  type DeleteStatementMutation,
  type MorphStatementMutation,
  type MoveStatementMutation,
  type RenameStatementMutation,
  type RestoreStatementMutation,
  type SetReferenceMutation,
  type SoftDeleteStatementMutation,
  type StatementModifier,
  type SymbolType,
  type UpdateStatementModifierMutation,
} from "@/gql/graphql";
import { useOperationsStore, type Transaction } from "@/state/operations";
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

const ALPHA_CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

export function newTypeNodeKey(): string {
  /* Generates an 8-character alphabetic random key :TypeNodeKeys */
  return Array.from({ length: 8 }, () => ALPHA_CHARS.charAt(Math.floor(Math.random() * ALPHA_CHARS.length))).join("");
}

export function newDatasetRecordId(): string {
  /* Generates a new dataset record global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`DatasetRecord:${nodeId}`);
}

export function useStatementOps() {
  const ops = useOperationsStore();
  const registry = new OpRegistry();

  const { mutate: createStatementMut } = registry.useMutation(
    ModuleMutationType.CreateStatement,
    graphql(/* GraphQL */ `
      mutation createStatement(
        $id: GlobalID
        $fileId: GlobalID!
        $parentId: GlobalID
        $orderKey: String!
        $type: StatementType!
        $modifier: StatementModifier
        $name: String
        $symbolType: SymbolType
        $lang: String
        $code: String
        $description: String
        $referenceId: GlobalID
        $rootTypeTag: TypeTag
        $commented: Boolean
        $generated: Boolean
      ) {
        createStatement(
          input: {
            id: $id
            fileId: $fileId
            parentId: $parentId
            orderKey: $orderKey
            type: $type
            modifier: $modifier
            name: $name
            symbolType: $symbolType
            lang: $lang
            code: $code
            description: $description
            rootTypeTag: $rootTypeTag
            referenceId: $referenceId
            commented: $commented
            generated: $generated
          }
        ) {
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
      optimisticResponse: (vars: {
        id: string;
        fileId: string;
        parentId: string | null;
        orderKey: string;
        type: StatementType;
        modifier: StatementModifier | null;
        name: string | null;
        symbolType: SymbolType | null;
        lang: string | null;
        code: string | null;
        description: string | null;
        referenceId: string | null;
        rootTypeTag: TypeTag | null;
        commented: boolean;
        generated: boolean;
      }) =>
        ({
          __typename: "Mutation",
          createStatement: {
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
            type: vars.type,
            modifier: vars.modifier,
            name: vars.name,
            symbolType: vars.symbolType,
            description: vars.description,
            code: vars.code,
            referenceProjectVersion: null,
            records: {
              totalCount: 0,
              edges: [],
            },
            rootTypeTag: vars.rootTypeTag,
            typeNodes: [],
            lang: vars.lang,
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
            generated: vars.generated,
            commented: vars.commented,
          },
        } as CreateStatementMutation),
      update(cache, { data }) {
        if (data?.createStatement.__typename != "Statement") {
          return; // error
        }
        // extend File.statements array with (ref to) new statement
        // must ensure that all relevant fields are present or weird things happen
        cache.modify({
          id: cache.identify(data.createStatement?.file),
          fields: {
            statements(currentStatements = []) {
              const newRef = cache.identify(data?.createStatement);
              return [
                ...currentStatements.filter((s: any) => s.__ref != newRef), // remove if already present
                { __ref: newRef },
              ];
            },
          },
          optimistic: true,
        });
      },
    }
  );

  async function createBlank(
    tx: Transaction | null,
    id: string,
    fileId: string,
    parentId: string | null,
    orderKey: string
  ) {
    return await ops.perform({
      tx,
      type: "statement.create",
      do: async () => {
        return await createStatementMut({
          id,
          fileId,
          parentId,
          orderKey,
          type: StatementType.Blank,
          modifier: null,
          name: null,
          symbolType: null,
          lang: null,
          code: null,
          description: null,
          referenceId: null,
          rootTypeTag: null,
          commented: false,
          generated: false,
        });
      },
      undo: async () => {
        return await softDeleteStatementMut({ id });
      },
      redo: async () => {
        return await restoreStatementMut({ id });
      },
    });
  }

  async function createDefinition(
    tx: Transaction | null,
    input: {
      id: string;
      fileId: string;
      parentId: string | null;
      orderKey: string;
      symbolType: SymbolType;
      name?: string;
      description?: string;
      rootTypeTag?: TypeTag;
    }
  ) {
    return await ops.perform({
      tx,
      type: "statement.create",
      do: async () => {
        return await createStatementMut({
          id: input.id,
          fileId: input.fileId,
          parentId: input.parentId,
          orderKey: input.orderKey,
          type: StatementType.Definition,
          modifier: null,
          name: input.name ?? null,
          symbolType: input.symbolType,
          lang: null,
          code: null,
          description: input.description ?? null,
          referenceId: null,
          rootTypeTag: input.rootTypeTag ?? null,
          commented: false,
          generated: false,
        });
      },
      undo: async () => {
        return await softDeleteStatementMut({ id: input.id });
      },
      redo: async () => {
        return await restoreStatementMut({ id: input.id });
      },
    });
  }

  const { mutate: morphStatementMut } = registry.useMutation(
    ModuleMutationType.MorphStatement,
    graphql(/* GraphQL */ `
      mutation morphStatement(
        $id: GlobalID!
        $type: StatementType!
        $symbolType: SymbolType
        $name: String
        $rootTypeTag: TypeTag
        $lang: String
      ) {
        morphStatement(
          input: { id: $id, type: $type, symbolType: $symbolType, name: $name, rootTypeTag: $rootTypeTag, lang: $lang }
        ) {
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
      optimisticResponse: (vars: {
        id: string;
        type: StatementType;
        symbolType?: SymbolType;
        name?: string;
        rootTypeTag?: TypeTag;
        lang?: string;
      }) =>
        ({
          morphStatement: {
            __typename: "Statement",
            id: vars.id,
            revision: PENDING_REVISION,
            type: vars.type,
            symbolType: vars.symbolType ?? null,
            name: vars.name ?? null,
            rootTypeTag: vars.rootTypeTag ?? null,
            lang: vars.lang ?? null,
          },
        } as MorphStatementMutation),
    }
  );

  async function morph(
    tx: Transaction | null,
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
    await ops.perform({
      tx,
      type: "statement.morph",
      do: async () => {
        return await morphStatementMut({ id, ...newStatement });
      },
      undo: async () => {
        return await morphStatementMut({ id, ...oldStatement });
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

  async function modify(
    tx: Transaction | null,
    id: string,
    oldModifier: StatementModifier | null,
    newModifier: StatementModifier | null
  ) {
    await ops.perform({
      tx,
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
    tx: Transaction | null,
    id: string,
    oldLoc: { fileId: string; parentId?: string; orderKey: string },
    newLoc: { fileId: string; parentId?: string; orderKey: string }
  ) {
    await ops.perform({
      tx,
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
    tx: Transaction | null,
    ids: string[],
    oldLocs: { fileId: string; parentId?: string; orderKey: string }[],
    newLocs: { fileId: string; parentId?: string; orderKey: string }[]
  ) {
    await ops.perform({
      tx,
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

  async function rename(tx: Transaction | null, id: string, oldName: string | null, newName: string | null) {
    await ops.perform({
      tx,
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

  async function delete_(tx: Transaction | null, id: string) {
    await ops.perform({
      tx,
      type: "statement.delete",
      do: async () => {
        return await deleteStatementMut({ id: id });
      },
    });
  }

  async function softDelete(tx: Transaction | null, id: string) {
    await ops.perform({
      tx,
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
    await ops.perform({
      type: "statement.batchDelete",
      do: async () => {
        return await batchSoftDeleteStatementMut({ ids: ids });
      },
      undo: async () => {
        return await batchRestoreStatementMut({ ids: ids });
      },
    });
  }

  async function restore(tx: Transaction | null, id: string) {
    await ops.perform({
      tx,
      type: "statement.restore",
      do: async () => {
        return await restoreStatementMut({ id: id });
      },
      undo: async () => {
        return await softDeleteStatementMut({ id: id });
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
    await ops.perform({
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

  async function comment(tx: Transaction | null, id: string, commented: boolean) {
    await ops.perform({
      tx,
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
    tx: Transaction | null,
    id: string,
    oldReferenceId: string | null,
    oldReferenceName: string | null,
    newReferenceId: string | null,
    newReferenceName: string | null
  ) {
    await ops.perform({
      tx,
      type: "statement.setReference",
      do: async () => {
        return await setReferenceMut({ id: id, referenceId: newReferenceId, referenceName: newReferenceName });
      },
      undo: async () => {
        return await setReferenceMut({ id: id, referenceId: oldReferenceId, referenceName: oldReferenceName });
      },
    });
  }

  return {
    registry,
    create: createBlank,
    createDefinition,
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
    restore,
    batchSoftDelete: batchSoftDelete_,
  };
}
