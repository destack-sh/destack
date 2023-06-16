import { graphql } from "@/gql";
import {
  ModuleMutationType,
  TypeHint,
  TypeTag,
  type BatchRestoreRecordMutation,
  type BatchSoftDeleteRecordMutation,
  type CreateRecordMutation,
  type DeleteRecordMutation,
  type DeleteFieldMutation,
  type RestoreRecordMutation,
  type RestoreFieldMutation,
  type Scalars,
  type SoftDeleteRecordMutation,
  type SoftDeleteFieldMutation,
  type FieldCreateInput,
  type FieldUpdateInput,
  type UpdateRecordMutation,
  type UpdateSymbolCodeMutation,
  type UpdateSymbolDescriptionMutation,
  type UpdateStatementTextMutation,
  type UpdateFieldMutation,
  type Field,
  type UpdateSymbolValueMutation,
} from "@/gql/graphql";
import { useOperationsStore, type Transaction } from "@/state/operations";
import { OpRegistry, PENDING_REVISION } from "@/state/sync";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolContentOps() {
  const ops = useOperationsStore();
  const registry = new OpRegistry();

  // symbol content mutations

  const { mutate: updateSymbolDescriptionMut } = registry.useMutation(
    ModuleMutationType.UpdateSymbolDescription,
    graphql(/* GraphQL */ `
      mutation updateSymbolDescription($id: GlobalID!, $description: String!) {
        updateSymbolDescription(input: { id: $id, description: $description }) {
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
          updateSymbolDescription: {
            __typename: "Statement",
            id: vars.id,
            description: vars.description,
            revision: PENDING_REVISION,
          },
        } as UpdateSymbolDescriptionMutation),
    }
  );

  async function updateSymbolDescription(
    tx: Transaction | null,
    id: string,
    oldDescription: string,
    newDescription: string
  ) {
    await ops.perform({
      tx,
      type: "statement.updateDescription",
      do: async () => {
        return await updateSymbolDescriptionMut({ id: id, description: newDescription });
      },
      undo: async () => {
        return await updateSymbolDescriptionMut({ id: id, description: oldDescription });
      },
    });
  }

  // code mutations

  const { mutate: updateSymbolCodeMut } = registry.useMutation(
    ModuleMutationType.UpdateSymbolCode,
    graphql(/* GraphQL */ `
      mutation updateSymbolCode($id: GlobalID!, $code: String) {
        updateSymbolCode(input: { id: $id, code: $code }) {
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
          updateSymbolCode: {
            __typename: "Statement",
            id: vars.id,
            code: vars.code,
            revision: PENDING_REVISION,
          },
        } as UpdateSymbolCodeMutation),
    }
  );

  async function updateSymbolCode(tx: Transaction | null, id: string, oldCode: string, newCode: string) {
    await ops.perform({
      tx,
      type: "statement.updateCode",
      do: async () => {
        return await updateSymbolCodeMut({ id, code: newCode });
      },
      undo: async () => {
        return await updateSymbolCodeMut({ id, code: oldCode });
      },
    });
  }

  const { mutate: updateStatementTextMut } = registry.useMutation(
    ModuleMutationType.UpdateStatementText,
    graphql(/* GraphQL */ `
      mutation updateStatementText($id: GlobalID!, $text: String) {
        updateStatementText(input: { id: $id, text: $text }) {
          ... on Statement {
            id
            text
            revision
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; text: string }) =>
        ({
          updateStatementText: {
            __typename: "Statement",
            id: vars.id,
            text: vars.text,
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
        return await updateStatementTextMut({ id, text: newCode });
      },
      undo: async () => {
        return await updateStatementTextMut({ id, text: oldCode });
      },
    });
  }

  const { mutate: updateValueMut } = registry.useMutation(
    ModuleMutationType.UpdateSymbolValue,
    graphql(/* GraphQL */ `
      mutation updateSymbolValue($id: GlobalID!, $value: JSON) {
        updateSymbolValue(input: { id: $id, value: $value }) {
          ... on Statement {
            id
            value
            revision
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; value: any }) =>
        ({
          updateSymbolValue: {
            __typename: "Statement",
            id: vars.id,
            value: vars.value,
            revision: PENDING_REVISION,
          },
        } as UpdateSymbolValueMutation),
    }
  );

  async function updateValue(tx: Transaction | null, id: string, oldValue: any, newValue: any) {
    await ops.perform({
      tx,
      type: "statement.updateValue",
      do: async () => {
        return await updateValueMut({ id, value: newValue });
      },
      undo: async () => {
        return await updateValueMut({ id, value: oldValue });
      },
    });
  }

  // record mutations

  const { mutate: createRecordMut } = registry.useMutation(
    ModuleMutationType.CreateRecord,
    graphql(/* GraphQL */ `
      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {
        createRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {
          ... on Record {
            id
            createdAt
            updatedAt
            deletedAt
            revision
            orderKey
            data
            statementId
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
            __typename: "Record",
            id: vars.id,
            statementId: vars.statementId,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            revision: PENDING_REVISION,
            orderKey: vars.orderKey,
            data: vars.data,
          },
        } as CreateRecordMutation),
    }
  );

  const { mutate: updateRecordMut } = registry.useMutation(
    ModuleMutationType.UpdateRecord,
    graphql(/* GraphQL */ `
      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {
        updateRecord(input: { id: $id, statementId: $statementId, data: $data }) {
          ... on Record {
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
            __typename: "Record",
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
      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {
        deleteRecord(input: { id: $id, statementId: $statementId }) {
          ... on Record {
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
            __typename: "Record",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as DeleteRecordMutation),
    }
  );

  const { mutate: softDeleteRecordMut } = registry.useMutation(
    ModuleMutationType.SoftDeleteRecord,
    graphql(/* GraphQL */ `
      mutation softDeleteRecord($id: GlobalID!, $statementId: GlobalID!) {
        softDeleteRecord(input: { id: $id, statementId: $statementId }) {
          ... on Record {
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
            __typename: "Record",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as SoftDeleteRecordMutation),
    }
  );

  const { mutate: restoreRecordMut } = registry.useMutation(
    ModuleMutationType.RestoreRecord,
    graphql(/* GraphQL */ `
      mutation restoreRecord($id: GlobalID!, $statementId: GlobalID!) {
        restoreRecord(input: { id: $id, statementId: $statementId }) {
          ... on Record {
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
            __typename: "Record",
            id: vars.id,
            deletedAt: null,
          },
        } as RestoreRecordMutation),
    }
  );

  const { mutate: batchSoftDeleteRecordMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation batchSoftDeleteRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {
        batchSoftDeleteRecord(input: { ids: $ids, statementId: $statementId }) {
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
              __typename: "Record",
              id: id,
              deletedAt: new Date().toISOString(),
            })),
          },
        } as BatchSoftDeleteRecordMutation),
    }
  );

  const { mutate: batchRestoreRecordMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation batchRestoreRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {
        batchRestoreRecord(input: { ids: $ids, statementId: $statementId }) {
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
              __typename: "Record",
              id: id,
              deletedAt: null,
            })),
          },
        } as BatchRestoreRecordMutation),
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
        return await softDeleteRecordMut({ statementId, id });
      },
      redo: async () => {
        return await restoreRecordMut({ statementId, id });
      },
    });
  }

  async function updateRecord(
    tx: Transaction | null,
    statementId: string,
    id: string,
    oldData: Scalars["JSON"],
    newData: Scalars["JSON"]
  ) {
    await ops.perform({
      tx,
      type: "statement.updateRecord",
      do: async () => {
        return await updateRecordMut({ statementId, id, data: newData });
      },
      undo: async () => {
        return await updateRecordMut({ statementId, id, data: oldData });
      },
    });
  }

  async function deleteRecord(tx: Transaction | null, statementId: string, id: string) {
    await ops.perform({
      tx,
      type: "statement.deleteRecord",
      do: async () => {
        return await deleteRecordMut({ statementId, id });
      },
    });
  }

  async function softDeleteRecord(tx: Transaction | null, statementId: string, id: string) {
    await ops.perform({
      tx,
      type: "statement.softDeleteRecord",
      do: async () => {
        return await softDeleteRecordMut({ statementId, id });
      },
      undo: async () => {
        return await restoreRecordMut({ statementId, id });
      },
    });
  }

  async function batchSoftDeleteRecord(statementId: string, ids: string[]) {
    await ops.perform({
      type: "statement.batchSoftDeleteRecord",
      do: async () => {
        return await batchSoftDeleteRecordMut({ statementId, ids });
      },
      undo: async () => {
        return await batchRestoreRecordMut({ statementId, ids });
      },
    });
  }

  async function batchRestoreRecord(statementId: string, ids: string[]) {
    await ops.perform({
      type: "statement.batchRestoreRecord",
      do: async () => {
        return await batchRestoreRecordMut({ statementId, ids });
      },
      undo: async () => {
        return await batchSoftDeleteRecordMut({ statementId, ids });
      },
    });
  }

  const { mutate: createFieldMut } = registry.useMutation(
    ModuleMutationType.CreateField,
    graphql(/* GraphQL */ `
      mutation createField(
        $id: GlobalID!
        $statementId: GlobalID!
        $tag: TypeTag!
        $hint: TypeHint
        $key: String!
        $orderKey: String!
        $name: String!
        $description: String
        $flags: Int!
        $referenceId: GlobalID
      ) {
        createField(
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
            referenceId: $referenceId
          }
        ) {
          ... on Field {
            # should match FieldContent fragment
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
        referenceId: string | null;
      }) =>
        ({
          __typename: "Mutation",
          createField: {
            __typename: "Field",
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
            orderKey: vars.orderKey,
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
            flags: vars.flags,
          },
        } as any),
      update(cache, { data }) {
        const createStatementField = data?.createField;
        if (createStatementField?.__typename != "Field") {
          return; // error
        }
        // extend Statement.fields with (ref to) new type node
        cache.modify({
          id: cache.identify(createStatementField.statement),
          fields: {
            fields(existingFields = []) {
              const newRef = cache.identify(createStatementField);
              return [
                ...existingFields.filter((t: any) => t.__ref != newRef), // remove old type node if exists
                { __ref: newRef },
              ];
            },
          },
          optimistic: true,
        });
      },
    }
  );

  const { mutate: deleteFieldMut } = registry.useMutation(
    ModuleMutationType.DeleteField,
    graphql(/* GraphQL */ `
      mutation deleteField($id: GlobalID!) {
        deleteField(input: { id: $id }) {
          ... on Field {
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
          deleteField: {
            __typename: "Field",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as DeleteFieldMutation),
    }
  );

  const { mutate: softDeleteFieldMut } = registry.useMutation(
    ModuleMutationType.SoftDeleteField,
    graphql(/* GraphQL */ `
      mutation softDeleteField($id: GlobalID!) {
        softDeleteField(input: { id: $id }) {
          ... on Field {
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
          softDeleteField: {
            __typename: "Field",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as SoftDeleteFieldMutation),
    }
  );

  const { mutate: restoreFieldMut } = registry.useMutation(
    ModuleMutationType.RestoreField,
    graphql(/* GraphQL */ `
      mutation restoreField($id: GlobalID!) {
        restoreStatementField(input: { id: $id }) {
          ... on Field {
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
          restoreStatementField: {
            __typename: "Field",
            id: vars.id,
            deletedAt: null,
          },
        } as RestoreFieldMutation),
    }
  );

  function _toFieldInput(
    statementId: string,
    input: Pick<
      Field,
      | "id"
      | "key"
      | "name"
      | "description"
      | "tag"
      | "hint"
      | "orderKey"
      | "flags"
      | "metadata"
      | "reference"
      | "statement"
    >
  ) {
    return {
      ...input,
      statementId,
      // set optional values to null if not provided
      hint: input.hint ?? null,
      description: input.description ?? null,
      referenceId: input.reference?.id ?? null,
      flags: input.flags ?? 0,
    } as FieldCreateInput;
  }

  async function createField(
    tx: Transaction | null,
    statementId: string,
    field: Pick<
      Field,
      | "id"
      | "name"
      | "tag"
      | "hint"
      | "description"
      | "key"
      | "orderKey"
      | "flags"
      | "metadata"
      | "reference"
      | "statement"
    >
  ) {
    await ops.perform({
      tx,
      type: "symbol.createField",
      do: async () => {
        return await createFieldMut(_toFieldInput(statementId, field));
      },
      undo: async () => {
        return await softDeleteFieldMut({ id: field.id });
      },
      redo: async () => {
        return await restoreFieldMut({ id: field.id });
      },
    });
  }

  async function deleteField(tx: Transaction | null, statementId: string, field: Pick<Field, "id">) {
    await ops.perform({
      tx,
      type: "symbol.deleteField",
      do: async () => {
        return await deleteFieldMut({ id: field.id });
      },
    });
  }

  async function softDeleteField(tx: Transaction | null, statementId: string, field: Pick<Field, "id">) {
    await ops.perform({
      tx,
      type: "symbol.softDeleteField",
      do: async () => {
        return await softDeleteFieldMut({ id: field.id });
      },
      undo: async () => {
        return await restoreFieldMut({ id: field.id });
      },
    });
  }

  const { mutate: updateFieldMut } = registry.useMutation(
    ModuleMutationType.UpdateField,
    graphql(/* GraphQL */ `
      mutation updateField(
        $id: GlobalID!
        $tag: TypeTag!
        $hint: TypeHint
        $name: String
        $description: String
        $flags: Int!
        $referenceId: GlobalID
      ) {
        updateField(
          input: {
            id: $id
            tag: $tag
            hint: $hint
            name: $name
            description: $description
            flags: $flags
            referenceId: $referenceId
          }
        ) {
          ... on Field {
            id
            tag
            hint
            updatedAt
            revision
            name
            description
            flags
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
        referenceId?: string;
      }) => {
        return {
          updateField: {
            __typename: "Field",
            id: vars.id,
            tag: vars.tag,
            hint: vars.hint ?? null,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            name: vars.name,
            description: vars.description,
            flags: vars.flags,
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
          },
        } as UpdateFieldMutation;
      },
    }
  );

  async function updateField(tx: Transaction | null, oldField: FieldUpdateInput, newField: FieldUpdateInput) {
    await ops.perform({
      tx,
      type: "symbol.updateField",
      do: async () => {
        return await updateFieldMut(newField);
      },
      undo: async () => {
        return await updateFieldMut(oldField);
      },
    });
  }

  const { mutate: moveFieldMut } = registry.useMutation(
    ModuleMutationType.MoveField,
    graphql(/* GraphQL */ `
      mutation moveField($id: GlobalID!, $orderKey: String!) {
        moveField(input: { id: $id, orderKey: $orderKey }) {
          ... on Field {
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
          moveField: {
            __typename: "Field",
            id: vars.id,
            orderKey: vars.orderKey,
          },
        } as any),
    }
  );

  async function moveField(tx: Transaction | null, id: string, oldOrderKey: string, newOrderKey: string) {
    await ops.perform({
      tx,
      type: "symbol.moveField",
      do: async () => {
        return await moveFieldMut({ id, orderKey: newOrderKey });
      },
      undo: async () => {
        return await moveFieldMut({ id, orderKey: oldOrderKey });
      },
    });
  }

  return {
    registry,
    updateSymbolDescription,
    updateSymbolCode,
    updateStatementText,
    updateValue,
    createRecord,
    updateRecord,
    deleteRecord,
    softDeleteRecord,
    batchSoftDeleteRecord,
    batchRestoreRecord,
    createField,
    updateField,
    moveField,
    deleteField,
    softDeleteField,
  };
}
