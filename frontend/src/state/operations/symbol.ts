import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolContentOps() {
  const operations = useOperationsStore();

  // type mutations

  const { mutate: updateStatementDescriptionMut } = useMutation(
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
    `)
  );

  async function updateStatementDescription(id: string, oldDescription: string, newDescription: string) {
    await operations.perform({
      type: "statement.updateDescription",
      do: async () => {
        await updateStatementDescriptionMut({ id: id, description: newDescription });
      },
      undo: async () => {
        await updateStatementDescriptionMut({ id: id, description: oldDescription });
      },
    });
  }

  // code mutations

  const { mutate: updateStatementCodeMut } = useMutation(
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
    `)
  );

  async function updateStatementCode(id: string, oldCode: string, newCode: string) {
    await operations.perform({
      type: "statement.updateCode",
      do: async () => {
        await updateStatementCodeMut({
          id: id,
          code: newCode,
        });
      },
      undo: async () => {
        await updateStatementCodeMut({
          id: id,
          code: oldCode,
        });
      },
    });
  }

  // dataset mutations (aka records)

  const { mutate: updateStatementRecordsMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateStatementRecords($id: GlobalID!, $records: [JSON!]!) {
        updateStatementRecords(input: { id: $id, records: $records }) {
          ... on Statement {
            id
            revision
            records {
              data
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function updateStatementRecords(id: string, oldRecords: Array<JSON>, newRecords: Array<JSON>) {
    await operations.perform({
      type: "statement.updateRecords",
      do: async () => {
        await updateStatementRecordsMut({ id: id, records: newRecords });
      },
      undo: async () => {
        await updateStatementRecordsMut({ id: id, records: oldRecords });
      },
    });
  }

  return { updateStatementDescription, updateStatementCode, updateStatementRecords };
}
