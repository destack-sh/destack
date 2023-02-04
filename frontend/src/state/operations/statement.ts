import { graphql, useFragment } from "@/gql";
import type { StatementModifier, StatementType, SymbolType } from "@/gql/graphql";
import { StatementHeaderType } from "@/state/fragments";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";
import { v4 as uuidv4 } from "uuid";

export function newStatementId(): string {
  /* Generates a new statement global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`Statement:${nodeId}`);
}

export function useStatementOps() {
  const operations = useOperationsStore();

  const { mutate: createStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createStatement(
        $id: GlobalID
        $fileId: GlobalID!
        $parentId: GlobalID
        $index: Int
        $type: StatementType!
        $name: String
      ) {
        createStatement(
          input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index, type: $type, name: $name }
        ) {
          ... on Statement {
            id
            ...StatementHeader
            text
            revision
            file {
              id
              path
              # should match FileInterface query
              statements(filters: { isVisible: true }) {
                id
                index
              }
            }
            parent {
              id
            }
          }
        }
      }
    `)
  );

  const { mutate: morphStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation morphStatement($id: GlobalID!, $type: StatementType!, $symbolType: SymbolType) {
        morphStatement(input: { id: $id, type: $type, symbolType: $symbolType }) {
          ... on Statement {
            id
            ...StatementHeader
            text
            code
            codeBuiltinId
            description
            typeNodes {
              ...TypeNodeData
            }
            revision
          }
          ...OperationInfoContent
        }
      }
    `)
  );

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
    `)
  );

  const { mutate: moveStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {
        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {
          ... on Statement {
            id
            index
            revision
            file {
              id
              path
              # should match FileInterface query
              statements(filters: { isVisible: true }) {
                id
                index
              }
            }
            parent {
              id
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  const { mutate: renameStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation renameStatement($id: GlobalID!, $name: String) {
        renameStatement(input: { id: $id, name: $name }) {
          ... on Statement {
            id
            name
            revision
            referencedBy {
              id
              name
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  const { mutate: deleteStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation deleteStatement($id: GlobalID!) {
        softDeleteStatement(input: { id: $id }) {
          ... on Statement {
            id
            deletedAt
            revision
            descendants {
              id
              deletedAt
            }
            # update all indices of statements in the same file
            file {
              id
              statements(filters: { isVisible: true }) {
                id
                index
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  const { mutate: restoreStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation restoreStatement($id: GlobalID!) {
        restoreStatement(input: { id: $id }) {
          ... on Statement {
            id
            deletedAt
            revision
            descendants {
              id
              deletedAt
            }
            # update all indices of statements in the same file
            file {
              id
              statements(filters: { isVisible: true }) {
                id
                index
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

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

  async function create(
    id: string,
    fileId: string,
    parentId: string | null,
    index: number | null,
    type: StatementType,
    name?: string
  ) {
    return await operations.perform({
      type: "statement.create",
      do: async () => {
        const create = await createStatementMut({
          id,
          fileId,
          parentId,
          index,
          type,
          name,
        });
        if (create?.data?.createStatement == null || create?.data?.createStatement.__typename !== "Statement") {
          // TODO @Robustness: unify error response handling
          throw new Error("invalid response");
        }
        return useFragment(StatementHeaderType, create?.data?.createStatement);
      },
      undo: async () => {
        await deleteStatementMut({ id });
      },
    });
  }

  async function morph(
    id: string,
    oldStatement: { type: StatementType; symbolType?: SymbolType },
    newStatement: { type: StatementType; symbolType?: SymbolType }
  ) {
    await operations.perform({
      type: "statement.morph",
      do: async () => {
        await morphStatementMut({
          id: id,
          type: newStatement.type,
          symbolType: newStatement.symbolType,
        });
      },
      undo: async () => {
        await morphStatementMut({
          id: id,
          type: oldStatement.type,
          symbolType: oldStatement.symbolType,
        });
      },
    });
  }

  async function modify(id: string, oldModifier: StatementModifier | null, newModifier: StatementModifier | null) {
    await operations.perform({
      type: "statement.modify",
      do: async () => {
        await updateStatementModifier({ id: id, modifier: newModifier });
      },
      undo: async () => {
        await updateStatementModifier({ id: id, modifier: oldModifier });
      },
    });
  }

  async function setReference(id: string, oldReferenceId: string | null, newReferenceId: string | null) {
    await operations.perform({
      type: "statement.setReference",
      do: async () => {
        await setReferenceMut({ id: id, referenceId: newReferenceId });
      },
      undo: async () => {
        await setReferenceMut({ id: id, referenceId: oldReferenceId });
      },
    });
  }

  async function move(
    id: string,
    oldLoc: { fileId: string; parentId?: string; index?: number },
    newLoc: { fileId: string; parentId?: string; index?: number }
  ) {
    await operations.perform({
      type: "statement.move",
      do: async () => {
        await moveStatementMut({
          id: id,
          fileId: newLoc.fileId,
          parentId: newLoc.parentId,
          index: newLoc.index,
        });
      },
      undo: async () => {
        await moveStatementMut({
          id: id,
          fileId: oldLoc.fileId,
          parentId: oldLoc.parentId,
          index: oldLoc.index,
        });
      },
    });
  }

  async function comment(id: string, commented: boolean) {
    await operations.perform({
      type: "statement.comment",
      do: async () => {
        await commentStatementMut({ id: id, commented: commented });
      },
      undo: async () => {
        await commentStatementMut({ id: id, commented: !commented });
      },
    });
  }

  async function rename(id: string, oldName: string | null, newName: string | null) {
    await operations.perform({
      type: "statement.rename",
      do: async () => {
        await renameStatementMut({ id: id, name: newName });
      },
      undo: async () => {
        await renameStatementMut({ id: id, name: oldName });
      },
    });
  }

  async function delete_(id: string) {
    await operations.perform({
      type: "statement.delete",
      do: async () => {
        await deleteStatementMut({ id: id });
      },
      undo: async () => {
        await restoreStatementMut({ id: id });
      },
    });
  }

  return { create, morph, modify, setReference, move, comment, rename, delete: delete_ };
}
