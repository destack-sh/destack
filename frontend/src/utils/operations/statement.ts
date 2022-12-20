import { graphql, useFragment } from "@/gql";
import type { StatementModifier, StatementType, SymbolType } from "@/gql/graphql";
import { StatementHeaderType } from "@/utils/fragments";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useStatementOps() {
  const operations = useOperationsStore();

  const { mutate: createStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createStatement(
        $fileId: GlobalID!
        $parentId: GlobalID
        $index: Int
        $type: StatementType!
        $name: String
      ) {
        createStatement(input: { fileId: $fileId, parentId: $parentId, index: $index, type: $type, name: $name }) {
          statement {
            id
            ...StatementHeader
            text
            file {
              id
              path
              statements {
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
    `),
    { refetchQueries: ["fileContentById"] }
  );

  const { mutate: morphStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation morphStatement($id: GlobalID!, $type: StatementType!, $symbolType: SymbolType) {
        morphStatement(input: { statementId: $id, type: $type, symbolType: $symbolType }) {
          statement {
            id
            ...StatementHeader
            text
            content {
              # not re-using symbol content fragment because that led to weird apollo errors
              ... on Code {
                ...CodeContent
              }
              ... on Dataset {
                ...DatasetContent
              }
              ... on Expectation {
                ...ExpectationContent
              }
              ... on Task {
                ...TaskContent
              }
              ... on Schema {
                ...SchemaContent
              }
            }
          }
        }
      }
    `)
  );

  const { mutate: setModifierStatement } = useMutation(
    graphql(/* GraphQL */ `
      mutation setModifierStatement($id: GlobalID!, $modifier: StatementModifier) {
        setModifierStatement(input: { id: $id, modifier: $modifier }) {
          ... on Statement {
            id
            modifier
          }
        }
      }
    `)
  );

  const { mutate: moveStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {
        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {
          statement {
            id
            index
            file {
              id
              path
            }
            parent {
              id
            }
          }
          oldFile {
            id
            path
            statements {
              id
              index
            }
          }
          newFile {
            id
            path
            statements {
              id
              index
            }
          }
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
          statement {
            id
            deletedAt
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
        }
      }
    `)
  );

  const { mutate: restoreStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation restoreStatement($id: GlobalID!) {
        restoreStatement(input: { id: $id }) {
          statement {
            id
            deletedAt
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
        }
      }
    `)
  );

  const { mutate: commentStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {
        commentStatement(input: { id: $id, commented: $commented }) {
          statement {
            id
            commented
            descendants {
              id
              commented
            }
          }
        }
      }
    `)
  );

  const { mutate: setReferenceMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {
        setReferenceStatement(input: { statementId: $id, referenceId: $referenceId }) {
          statement {
            id
            reference {
              ...StatementHeader
            }
          }
        }
      }
    `)
  );

  async function create(fileId: string, parentId: string | null, index: number, type: StatementType, name?: string) {
    return await operations.perform({
      type: "statement.create",
      do: async () => {
        const create = await createStatementMut({
          fileId: fileId,
          parentId: parentId,
          index: index,
          type: type,
          name: name,
        });
        const statement = create?.data?.createStatement.statement;
        if (statement == null) {
          throw new Error("invalid response");
        }
        return useFragment(StatementHeaderType, statement);
      },
      undo: async (statement) => {
        await deleteStatementMut({ id: statement.id });
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
        await setModifierStatement({ id: id, modifier: newModifier });
      },
      undo: async () => {
        await setModifierStatement({ id: id, modifier: oldModifier });
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
