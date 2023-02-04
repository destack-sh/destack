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
        $orderKey: String!
        $type: StatementType!
        $name: String
      ) {
        createStatement(
          input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey, type: $type, name: $name }
        ) {
          ... on Statement {
            ...StatementContent
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
    orderKey: string,
    type: StatementType,
    name?: string
  ) {
    async function apply() {
      const create = await createStatementMut(
        {
          id,
          fileId,
          parentId,
          orderKey,
          type,
          name,
        },
        {
          update(cache, { data: createStatement }) {
            console.log("update existing statements", createStatement);
            cache.modify({
              id: `File:${fileId}`,
              fields: {
                statements(currentStatements = []) {
                  return [...currentStatements, createStatement?.createStatement];
                },
              },
            });
          },
        }
      );
      if (create?.data?.createStatement == null || create?.data?.createStatement.__typename !== "Statement") {
        // TODO @Robustness: unify error response handling
        throw new Error("invalid response");
      }
      return useFragment(StatementHeaderType, create?.data?.createStatement);
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

  const { mutate: moveStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {
        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {
          ... on Statement {
            id
            orderKey
            revision
            parent {
              id
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; fileId: string; parentId: string; orderKey: string }) => ({
        __typename: "Statement",
        id: vars.id,
        orderKey: vars.id,
        file: {
          id: vars.fileId,
        },
        parent: {
          id: vars.parentId,
        },
      }),
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
        await moveStatementMut({
          id: id,
          fileId: newLoc.fileId,
          parentId: newLoc.parentId,
          orderKey: newLoc.orderKey,
        });
      },
      undo: async () => {
        await moveStatementMut({
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
    `)
  );

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
      optimisticResponse: (vars: { id: string }) => ({
        softDeleteStatement: {
          __typename: "Statement",
          id: vars.id,
          deletedAt: new Date().toISOString(),
          descendants: [], // unknown
        },
      }),
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
      optimisticResponse: (vars: { id: string }) => ({
        restoreStatement: {
          __typename: "Statement",
          id: vars.id,
          deletedAt: null,
          descendants: [], // unknown
        },
      }),
    }
  );

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
        await commentStatementMut({ id: id, commented: commented });
      },
      undo: async () => {
        await commentStatementMut({ id: id, commented: !commented });
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
        await setReferenceMut({ id: id, referenceId: newReferenceId });
      },
      undo: async () => {
        await setReferenceMut({ id: id, referenceId: oldReferenceId });
      },
    });
  }

  return { create, morph, modify, setReference, move, comment, rename, delete: delete_ };
}
