import { graphql } from "@/gql";
import {
  StatementType,
  TypeTag,
  type CreateStatementMutation,
  type DeleteStatementMutation,
  type MorphStatementMutation,
  type MoveStatementMutation,
  type RenameStatementMutation,
  type RestoreStatementMutation,
  type StatementModifier,
  type StatementMorphInput,
  type SymbolType,
  type TypeNodeCreateInput,
  type TypeNodeUpdateInput,
  type UpdateStatementModifierMutation,
} from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
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

const PENDING_REVISION = -1;

export function useStatementOps() {
  const operations = useOperationsStore();

  const { mutate: createStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createStatement($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {
        createStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {
          ... on Statement {
            id
            type
            symbolType
            revision
            orderKey
            file {
              id
            }
            parent {
              id
            }
            ...StatementContent
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; fileId: string; parentId: string | null; orderKey: string }) =>
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
            revision: -1,
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
            records: [],
            rootTypeTag: null,
            typeNodes: [],
            lang: null,
            reference: null,
            importPath: null,
            generated: false,
            commented: false,
          },
        } as CreateStatementMutation),
      update(cache, { data: createStatement }) {
        if (createStatement?.createStatement.__typename != "Statement") {
          return; // error
        }
        // extend File.statements array with (ref to) new statement
        // must ensure that all relevant fields are present or weird things happen
        cache.modify({
          id: cache.identify(createStatement.createStatement?.file),
          fields: {
            statements(currentStatements = []) {
              return [...currentStatements, { __ref: cache.identify(createStatement?.createStatement) }];
            },
          },
          optimistic: true,
        });
      },
    }
  );

  async function create(id: string, fileId: string, parentId: string | null, orderKey: string) {
    async function apply() {
      return await createStatementMut({
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
        await deleteStatementMut({ id });
      },
    });
  }

  const { mutate: morphStatementMut } = useMutation(
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

  const { mutate: updateStatementModifier } = useMutation(
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

  const { mutate: moveStatementMut } = useMutation(
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

  const { mutate: renameStatementMut } = useMutation(
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

  const { mutate: deleteStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation deleteStatement($id: GlobalID!) {
        softDeleteStatement(input: { id: $id }) {
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
          softDeleteStatement: {
            __typename: "Statement",
            id: vars.id,
            deletedAt: new Date().toISOString(),
            descendants: [], // unknown
          },
        } as DeleteStatementMutation),
    }
  );

  const { mutate: restoreStatementMut } = useMutation(
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

  async function delete_(id: string) {
    await operations.perform({
      type: "statement.delete",
      do: async () => {
        return await deleteStatementMut({ id: id });
      },
      undo: async () => {
        return await restoreStatementMut({ id: id });
      },
    });
  }

  const { mutate: commentStatementMut } = useMutation(
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
    `)
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

  const { mutate: setReferenceMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {
        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {
          ... on Statement {
            id
            revision
            reference {
              ...StatementHeader
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function setReference(id: string, oldReferenceId: string | null, newReferenceId: string | null) {
    await operations.perform({
      type: "statement.setReference",
      do: async () => {
        return await setReferenceMut({ id: id, referenceId: newReferenceId });
      },
      undo: async () => {
        return await setReferenceMut({ id: id, referenceId: oldReferenceId });
      },
    });
  }

  // TODO @Performance: mutate type nodes optimistically  :SubSymbolRevisions
  const { mutate: createTypeNodeMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createTypeNode($typeNode: TypeNodeCreateInput!) {
        createStatementTypeNode(input: $typeNode) {
          ... on Statement {
            id
            revision
            typeNodes {
              ...SimpleTypeNodeContent
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  const { mutate: deleteTypeNodeMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation deleteTypeNode($id: GlobalID!, $statementId: GlobalID!) {
        deleteStatementTypeNode(input: { id: $id, statementId: $statementId }) {
          ... on Statement {
            id
            revision
            typeNodes {
              ...SimpleTypeNodeContent
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function createTypeNode(statementId: string, typeNode: TypeNodeCreateInput) {
    await operations.perform({
      type: "statement.createTypeNode",
      do: async () => {
        return await createTypeNodeMut({ typeNode: typeNode });
      },
      undo: async () => {
        return await deleteTypeNodeMut({ id: typeNode.id, statementId });
      },
    });
  }

  async function deleteTypeNode(statementId: string, typeNode: TypeNodeCreateInput) {
    await operations.perform({
      type: "statement.deleteTypeNode",
      do: async () => {
        return await deleteTypeNodeMut({ id: typeNode.id, statementId });
      },
      undo: async () => {
        return await createTypeNodeMut({ typeNode });
      },
    });
  }

  const { mutate: updateTypeNodeMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateTypeNode($typeNode: TypeNodeUpdateInput!) {
        updateStatementTypeNode(input: $typeNode) {
          ... on Statement {
            id
            revision
            typeNodes {
              ...SimpleTypeNodeContent
            }
          }
          ...OperationInfoContent
        }
      }
    `)
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
    create,
    morph,
    modify,
    setReference,
    move,
    comment,
    rename,
    delete: delete_,
    createTypeNode,
    updateTypeNode,
    deleteTypeNode,
  };
}
