import { graphql } from "@/gql";
import {
  EditType,
  StatementType,
  TypeTag,
  type BatchDeleteStatementsMutation,
  type BatchMoveStatementMutation,
  type BatchRestoreStatementsMutation,
  type CreateStatementMutation,
  type DeleteStatementMutation,
  type MorphStatementMutation,
  type MoveStatementMutation,
  type RenameStatementMutation,
  type RestoreStatementMutation,
  type SoftDeleteStatementMutation,
  type UpdateStatementMutation,
} from "@/gql/graphql";
import { useOperationsStore, type Transaction } from "@/state/operations";
import { EditRegistry, PENDING_REVISION } from "@/state/sync";
import { cyrb53a } from "@/utils/functools";
import { useMutation } from "@vue/apollo-composable";

export function newDynamicNodeKey(ck: string): string {
  /**
   * Gets a 'random' alphabetic key as a persistent key for a field.
   * (FIELD_KEY_LENGTH alphabetic characters)
   */

  let hashValue = cyrb53a(ck);
  let key = "";

  while (key.length < 8) {
    let remainder: number;
    [hashValue, remainder] = [Math.floor(hashValue / 52), hashValue % 52];

    // Map remainder to [a-zA-Z]
    if (remainder < 26) {
      key += String.fromCharCode(97 + remainder); // 'a'.charCodeAt(0) === 97
    } else {
      key += String.fromCharCode(65 + remainder - 26); // 'A'.charCodeAt(0) === 65
    }
  }

  return key;
}

export function useStatementOps() {
  const ops = useOperationsStore();
  const registry = new EditRegistry();

  // yeah there's some annoying redundance here, see :BE-114

  const { mutate: createStatementMut } = registry.defineEdit(
    EditType.CreateStatement,
    graphql(/* GraphQL */ `
      mutation createStatement(
        $id: GlobalID!
        $ck: UUID!
        $fileId: GlobalID!
        $parentId: GlobalID
        $orderKey: String!
        $type: StatementType!
        $name: String
        $key: String
        $code: String
        $text: String
        $value: JSON
        $tag: TypeTag
        $flags: Int
        $versioned: Boolean!
      ) {
        createStatement(
          input: {
            id: $id
            ck: $ck
            fileId: $fileId
            parentId: $parentId
            orderKey: $orderKey
            type: $type
            name: $name
            key: $key
            code: $code
            text: $text
            value: $value
            tag: $tag
            flags: $flags
            versioned: $versioned
          }
        ) {
          ... on Statement {
            # should match StatementContent fragment
            id
            ck
            type
            revision
            name
            orderKey
            file {
              id
            }
            parent {
              ... on Statement {
                id
              }
              ... on File {
                id
              }
            }
            # symbol contents
            key
            code
            text
            headingLevel
            value
            tag
            flags
            referenceCk
            versioned
            tags(filters: { isVisible: true }) {
              id
            }
            fields(filters: { isVisible: true }) {
              id
            }
            triggers(filters: { isVisible: true }) {
              id
            }
            # interp
            resolvedFields {
              statement {
                id
              }
              fieldCk
            }
            issues {
              id
            }
            # crud
            createdAt
            updatedAt
            deletedAt
            createdBy {
              id
            }
            lastEditedAt
            lastEditedBy {
              id
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: function (vars: {
        id: string;
        ck: string;
        fileId: string;
        parentId: string | null;
        orderKey: string;
        type: StatementType;
        name: string | null;
        key: string | null;
        code: string | null;
        text: string | null;
        value: any | null;
        tag: TypeTag | null;
        flags: number | null;
        versioned: boolean;
      }) {
        return {
          __typename: "Mutation",
          createStatement: {
            __typename: "Statement",
            id: vars.id,
            ck: vars.ck,
            file: {
              __typename: "File",
              id: vars.fileId,
            },
            parent:
              vars.parentId == null || atob(vars.parentId).startsWith("File:")
                ? { __typename: "File", id: vars.fileId }
                : { __typename: "Statement", id: vars.parentId },
            revision: PENDING_REVISION,
            orderKey: vars.orderKey,
            key: vars.key,
            type: vars.type,
            name: vars.name,
            value: vars.value,
            code: vars.code,
            text: vars.text,
            headingLevel: null,
            referenceCk: null,
            tag: vars.tag,
            flags: vars.flags,
            versioned: vars.versioned,
            tags: [],
            fields: [],
            triggers: [],
            // interp
            resolvedFields: [],
            issues: [],
            // crud
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            createdBy: null,
            lastEditedAt: new Date().toISOString(),
            lastEditedBy: null,
          },
        } as CreateStatementMutation;
      },
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
    ck: string,
    fileId: string,
    parentId: string | undefined | null,
    orderKey: string
  ) {
    return await ops.perform({
      tx,
      type: "statement.create",
      do: async () => {
        return await createStatementMut({
          id,
          ck,
          fileId,
          parentId: parentId ?? null,
          orderKey,
          type: StatementType.Blank,
          name: null,
          code: null,
          text: null,
          key: null,
          value: null,
          tag: null,
          flags: null,
          versioned: true,
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

  async function create(
    tx: Transaction | null,
    input: {
      id: string;
      ck: string;
      type: StatementType;
      fileId: string;
      parentId: string | undefined | null;
      orderKey: string;
      name?: string;
      key?: string;
      referenceCk?: string;
      text?: string;
      tag?: TypeTag;
      flags?: number;
      versioned?: boolean;
    }
  ) {
    return await ops.perform({
      tx,
      type: "statement.create",
      do: async () => {
        return await createStatementMut({
          id: input.id,
          ck: input.ck,
          fileId: input.fileId,
          parentId: input.parentId ?? null,
          orderKey: input.orderKey,
          type: input.type,
          name: input.name ?? null,
          code: null,
          value: null,
          key: input.key ?? null,
          text: input.text ?? null,
          tag: input.tag ?? null,
          flags: input.flags ?? null,
          versioned: input.versioned ?? true,
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

  /* not used in client, just for registration as a sync op */
  registry.defineEdit(
    EditType.UpdateStatement,
    graphql(/* GraphQL */ `
      mutation updateStatement(
        $id: GlobalID!
        $orderKey: String!
        $type: StatementType!
        $name: String
        $code: String
        $text: String
        $value: JSON
        $tag: TypeTag
        $flags: Int
      ) {
        updateStatement(
          input: {
            id: $id
            orderKey: $orderKey
            type: $type
            name: $name
            code: $code
            text: $text
            value: $value
            tag: $tag
            flags: $flags
          }
        ) {
          ... on Statement {
            # should match StatementContent fragment
            id
            type
            revision
            updatedAt
            name
            orderKey
            code
            text
            value
            tag
            flags
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
        name: string | null;
        code: string | null;
        key: string | null;
        text: string | null;
        value: any | null;
        tag: TypeTag | null;
        flags: number | null;
      }) =>
        ({
          __typename: "Mutation",
          updateStatement: {
            __typename: "Statement",
            id: vars.id,
            revision: PENDING_REVISION,
            orderKey: vars.orderKey,
            // default new fields (all! fields in StatementContent fragment)
            updatedAt: new Date().toISOString(),
            type: vars.type,
            name: vars.name,
            value: vars.value,
            code: vars.code,
            key: vars.key,
            text: vars.text,
            tag: vars.tag,
            flags: vars.flags,
            fields: [],
          },
        } as UpdateStatementMutation),
    }
  );

  const { mutate: morphStatementMut } = registry.defineEdit(
    EditType.MorphStatement,
    graphql(/* GraphQL */ `
      mutation morphStatement(
        $id: GlobalID!
        $type: StatementType!
        $name: String
        $tag: TypeTag
        $flags: Int
        $key: String
        $headingLevel: Int
        $versioned: Boolean!
      ) {
        morphStatement(
          input: {
            id: $id
            type: $type
            name: $name
            tag: $tag
            flags: $flags
            key: $key
            headingLevel: $headingLevel
            versioned: $versioned
          }
        ) {
          ... on Statement {
            id
            revision
            type
            name
            tag
            flags
            key
            headingLevel
            versioned
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        id: string;
        type: StatementType;
        name?: string;
        tag?: TypeTag;
        flags?: number;
        key?: string;
        headingLevel?: number;
        versioned: boolean;
      }) =>
        ({
          morphStatement: {
            __typename: "Statement",
            id: vars.id,
            revision: PENDING_REVISION,
            type: vars.type,
            name: vars.name ?? null,
            tag: vars.tag ?? null,
            flags: vars.flags ?? null,
            key: vars.key ?? null,
            headingLevel: vars.headingLevel ?? null,
            versioned: vars.versioned,
          },
        } as MorphStatementMutation),
    }
  );

  async function morph(
    tx: Transaction | null,
    id: string,
    oldStatement: {
      type: StatementType;
      name?: string | null;
      tag?: TypeTag | null;
      flags?: number | null;
      key?: string | null;
      headingLevel?: number | null;
      versioned?: boolean | null;
    },
    newStatement: {
      type: StatementType;
      name?: string | null;
      tag?: TypeTag | null;
      flags?: number | null;
      key?: string | null;
      headingLevel?: number | null;
      versioned?: boolean | null;
    }
  ) {
    const old = {
      id,
      type: oldStatement.type,
      name: oldStatement.name ?? undefined,
      tag: oldStatement.tag ?? undefined,
      flags: oldStatement.flags ?? undefined,
      key: oldStatement.key ?? undefined,
      headingLevel: oldStatement.headingLevel ?? undefined,
      versioned: oldStatement.versioned === undefined ? true : oldStatement.versioned ?? false,
    };
    await ops.perform({
      tx,
      type: "statement.morph",
      do: async () => {
        return await morphStatementMut({
          id,
          type: newStatement.type,
          name: newStatement.name ?? undefined,
          tag: newStatement.tag ?? undefined,
          flags: newStatement.flags ?? undefined,
          key: newStatement.key ?? undefined,
          headingLevel: newStatement.headingLevel ?? undefined,
          versioned: newStatement.versioned === undefined ? true : newStatement.versioned ?? false,
        });
      },
      undo: async () => {
        return await morphStatementMut(old);
      },
    });
  }

  const { mutate: moveStatementMut } = registry.defineEdit(
    EditType.MoveStatement,
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
              ... on Statement {
                id
              }
              ... on File {
                id
              }
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
            parent: vars.parentId
              ? { __typename: "Statement", id: vars.parentId }
              : { __typename: "File", id: vars.fileId },
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
                ... on Statement {
                  id
                }
                ... on File {
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
              parent: vars.parentIds[i]
                ? { __typename: "Statement", id: vars.parentIds[i] }
                : { __typename: "File", id: vars.fileId },
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

  const { mutate: renameStatementMut } = registry.defineEdit(
    EditType.RenameStatement,
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

  const { mutate: deleteStatementMut } = registry.defineEdit(
    EditType.DeleteStatement,
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

  const { mutate: softDeleteStatementMut } = registry.defineEdit(
    EditType.SoftDeleteStatement,
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

  const { mutate: restoreStatementMut } = registry.defineEdit(
    EditType.RestoreStatement,
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
        $targetCks: [UUID!]!
        $targetFileId: GlobalID!
        $targetParentIds: [GlobalID]!
        $targetOrderKeys: [String!]!
      ) {
        batchPasteStatement(
          input: {
            sourceIds: $sourceIds
            targetIds: $targetIds
            targetCks: $targetCks
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
    targetCks: string[],
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
          targetCks: targetCks,
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

  return {
    registry,
    create: createBlank,
    createDefinition: create,
    morph,
    move,
    batchMove,
    batchPaste,
    rename,
    delete: delete_,
    softDelete: softDelete,
    restore,
    batchSoftDelete: batchSoftDelete_,
  };
}
