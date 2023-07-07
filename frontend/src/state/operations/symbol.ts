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
  type TaggingUpdateInput,
  type Tagging,
} from "@/gql/graphql";
import { useOperationsStore, type Transaction } from "@/state/operations";
import { OpRegistry, PENDING_REVISION } from "@/state/sync";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolContentOps() {
  const ops = useOperationsStore();
  const registry = new OpRegistry();

  // symbol content mutations
  // (for the annoying redundancy see :BE-114)

  const { mutate: updateStatementReferenceMut } = registry.useMutation(
    ModuleMutationType.UpdateStatementReference,
    graphql(/* GraphQL */ `
      mutation updateStatementReference($id: GlobalID!, $referenceId: GlobalID!) {
        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {
          ... on Statement {
            id
            revision
            reference {
              id
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; referenceId: string | null }) =>
        ({
          updateStatementReference: {
            __typename: "Statement",
            id: vars.id,
            revision: PENDING_REVISION,
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
          },
        } as UpdateStatementReferenceMutation),
    }
  );

  async function updateStatementReference(
    tx: Transaction | null,
    id: string,
    oldReferenceId: string | null,
    newReferenceId: string | null
  ) {
    await ops.perform({
      tx,
      type: "statement.updateReference",
      do: async () => {
        return await updateStatementReferenceMut({ id, referenceId: newReferenceId });
      },
      undo: async () => {
        return await updateStatementReferenceMut({ id, referenceId: oldReferenceId });
      },
    });
  }

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
      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String, $value: JSON!) {
        createRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, value: $value }) {
          ... on Record {
            id
            createdAt
            updatedAt
            deletedAt
            revision
            orderKey
            value
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; statementId: string; orderKey: string; value: any }) =>
        ({
          __typename: "Mutation",
          createRecord: {
            __typename: "Record",
            id: vars.id,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            revision: PENDING_REVISION,
            orderKey: vars.orderKey,
            value: vars.value,
          },
        } as CreateRecordMutation),
    }
  );

  const { mutate: updateRecordMut } = registry.useMutation(
    ModuleMutationType.UpdateRecord,
    graphql(/* GraphQL */ `
      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $value: JSON!) {
        updateRecord(input: { id: $id, statementId: $statementId, value: $value }) {
          ... on Record {
            id
            updatedAt
            revision
            value
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; value: any }) =>
        ({
          updateRecord: {
            __typename: "Record",
            id: vars.id,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            value: vars.value,
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
    orderKey: string | null,
    value: Scalars["JSON"]
  ) {
    await ops.perform({
      tx,
      type: "statement.createRecord",
      do: async () => {
        return await createRecordMut({
          id: id,
          statementId: statementId,
          orderKey: orderKey,
          value: value,
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
    oldValue: Scalars["JSON"],
    newValue: Scalars["JSON"]
  ) {
    await ops.perform({
      tx,
      type: "statement.updateRecord",
      do: async () => {
        return await updateRecordMut({ statementId, id, value: newValue });
      },
      undo: async () => {
        return await updateRecordMut({ statementId, id, value: oldValue });
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
        $metadata: JSON
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
            metadata: $metadata
          }
        ) {
          ... on Field {
            # should match :FieldContent fragment
            id
            key
            orderKey
            statement {
              id
            }
            parent {
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
            metadata
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
        metadata: any;
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
            parent: {
              __typename: "Statement",
              id: vars.statementId,
            },
            revision: PENDING_REVISION,
            tag: vars.tag,
            hint: vars.hint ?? null,
            name: vars.name,
            key: vars.key,
            description: vars.description ?? null,
            orderKey: vars.orderKey,
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
            flags: vars.flags,
            metadata: vars.metadata ?? null,
            // crud
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            createdBy: null,
            lastEditedAt: null,
            lastEditedBy: null,
          },
        } as any),
      update(cache, { data }) {
        const createStatementField = data?.createField;
        if (createStatementField?.__typename != "Field") {
          return; // error
        }
        // extend Statement.fields with (ref to) new field
        cache.modify({
          id: cache.identify(createStatementField.statement),
          fields: {
            fields(existingFields = []) {
              const newRef = cache.identify(createStatementField);
              return [
                ...existingFields.filter((t: any) => t.__ref != newRef), // remove old field if exists
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
        $metadata: JSON
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
            metadata: $metadata
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
            metadata
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
        metadata?: any;
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
            metadata: vars.metadata ?? null,
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

  const { mutate: createTaggingMut } = registry.useMutation(
    ModuleMutationType.CreateTagging,
    graphql(/* GraphQL */ `
      mutation createTagging(
        $id: GlobalID!
        $statementId: GlobalID!
        $key: String!
        $referenceId: GlobalID!
        $metadata: JSON
      ) {
        createTagging(
          input: { id: $id, statementId: $statementId, key: $key, referenceId: $referenceId, metadata: $metadata }
        ) {
          ... on Tagging {
            id
            revision
            key
            parent {
              id
            }
            reference {
              id
            }
            metadata
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
      optimisticResponse: (vars: {
        id: string;
        statementId: string;
        key: string;
        referenceId: string | null;
        metadata: any;
      }) =>
        ({
          __typename: "Mutation",
          createTagging: {
            __typename: "Tagging",
            id: vars.id,
            revision: PENDING_REVISION,
            key: vars.key,
            parent: {
              __typename: "Statement",
              id: vars.statementId,
            },
            reference: vars.referenceId == null ? null : { __typename: "Statement", id: vars.referenceId },
            metadata: vars.metadata ?? null,
            // crud
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            createdBy: null,
            lastEditedAt: null,
            lastEditedBy: null,
          },
        } as any),
      update(cache, { data }) {
        const createTagging = data?.createTagging;
        if (createTagging?.__typename != "Tagging") {
          return; // error
        }
        // extend Statement.tags with (ref to) new tag
        cache.modify({
          id: cache.identify(createTagging.parent),
          fields: {
            tags(existingTags = []) {
              const newRef = cache.identify(createTagging);
              return [
                ...existingTags.filter((t: any) => t.__ref != newRef), // remove old tag if exists
                { __ref: newRef },
              ];
            },
          },
          optimistic: true,
        });
      },
    }
  );

  const { mutate: deleteTaggingMut } = registry.useMutation(
    ModuleMutationType.DeleteTagging,
    graphql(/* GraphQL */ `
      mutation deleteTagging($id: GlobalID!) {
        deleteTagging(input: { id: $id }) {
          ... on Tagging {
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
          deleteTagging: {
            __typename: "Tagging",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as any),
    }
  );

  const { mutate: softDeleteTaggingMut } = registry.useMutation(
    ModuleMutationType.SoftDeleteTagging,
    graphql(/* GraphQL */ `
      mutation softDeleteTagging($id: GlobalID!) {
        softDeleteTagging(input: { id: $id }) {
          ... on Tagging {
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
          softDeleteTagging: {
            __typename: "Tagging",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as any),
    }
  );

  const { mutate: restoreTaggingMut } = registry.useMutation(
    ModuleMutationType.RestoreTagging,
    graphql(/* GraphQL */ `
      mutation restoreTagging($id: GlobalID!) {
        restoreTagging(input: { id: $id }) {
          ... on Tagging {
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
          restoreTagging: {
            __typename: "Tagging",
            id: vars.id,
            deletedAt: null,
          },
        } as any),
    }
  );

  async function createTagging(
    tx: Transaction | null,
    statementId: string,
    tagging: Pick<Tagging, "id" | "key" | "reference" | "metadata">
  ) {
    await ops.perform({
      tx,
      type: "symbol.createTagging",
      do: async () => {
        return await createTaggingMut({
          id: tagging.id,
          statementId: statementId,
          key: tagging.key,
          referenceId: tagging.reference?.id ?? null,
          metadata: tagging.metadata ?? null,
        });
      },
      undo: async () => {
        return await softDeleteTaggingMut({ id: tagging.id });
      },
      redo: async () => {
        return await restoreTaggingMut({ id: tagging.id });
      },
    });
  }

  async function deleteTagging(tx: Transaction | null, statementId: string, tagging: Pick<Tagging, "id">) {
    await ops.perform({
      tx,
      type: "symbol.deleteTagging",
      do: async () => {
        return await deleteTaggingMut({ id: tagging.id });
      },
    });
  }

  async function softDeleteTagging(tx: Transaction | null, statementId: string, tagging: Pick<Tagging, "id">) {
    await ops.perform({
      tx,
      type: "symbol.softDeleteTagging",
      do: async () => {
        return await softDeleteTaggingMut({ id: tagging.id });
      },
      undo: async () => {
        return await restoreTaggingMut({ id: tagging.id });
      },
    });
  }

  const { mutate: updateTaggingMut } = registry.useMutation(
    ModuleMutationType.UpdateTagging,
    graphql(/* GraphQL */ `
      mutation updateTagging($id: GlobalID!, $metadata: JSON) {
        updateTagging(input: { id: $id, metadata: $metadata }) {
          ... on Tagging {
            id
            updatedAt
            revision
            metadata
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; key: string; referenceId: string | null; metadata: any }) =>
        ({
          updateTagging: {
            __typename: "Tagging",
            id: vars.id,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            metadata: vars.metadata ?? null,
          },
        } as any),
    }
  );

  async function updateTaggingMetadata(
    tx: Transaction | null,
    statementId: string,
    oldTagging: Pick<Tagging, "id" | "metadata">,
    newTagging: Pick<Tagging, "id" | "metadata">
  ) {
    await ops.perform({
      tx,
      type: "symbol.updateTaggingMetadata",
      do: async () => {
        return await updateTaggingMut(newTagging);
      },
      undo: async () => {
        return await updateTaggingMut(oldTagging);
      },
    });
  }

  return {
    registry,
    updateStatementReference,
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
    createTagging,
    updateTaggingMetadata,
    deleteTagging,
    softDeleteTagging,
  };
}
