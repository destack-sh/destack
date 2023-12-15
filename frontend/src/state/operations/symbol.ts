import { graphql } from "@/gql";
import {
  EditType,
  TypeHint,
  TypeTag,
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
  type UpdateStatementTextMutation,
  type UpdateFieldMutation,
  type Field,
  type UpdateSymbolValueMutation,
  type Tagging,
  TriggerType,
  type Trigger,
  ScheduleType,
} from "@/gql/graphql";
import { useOperationsStore, type Transaction } from "@/state/operations";
import { EditRegistry, PENDING_REVISION } from "@/state/sync";

export function useSymbolContentOps() {
  const ops = useOperationsStore();
  const registry = new EditRegistry();

  // symbol content mutations
  // (for the annoying redundancy see :BE-114)

  // code mutations

  const { mutate: updateSymbolCodeMut } = registry.defineEdit(
    EditType.UpdateSymbolCode,
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

  const { mutate: updateStatementTextMut } = registry.defineEdit(
    EditType.UpdateStatementText,
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

  async function updateStatementText(tx: Transaction | null, id: string, oldText: string, newText: string) {
    await ops.perform({
      tx,
      type: "statement.updateText",
      do: async () => {
        return await updateStatementTextMut({ id, text: newText });
      },
      undo: async () => {
        return await updateStatementTextMut({ id, text: oldText });
      },
    });
  }

  const { mutate: updateValueMut } = registry.defineEdit(
    EditType.UpdateSymbolValue,
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

  const { mutate: createRecordMut } = registry.defineEdit(
    EditType.CreateRecord,
    graphql(/* GraphQL */ `
      mutation createRecord(
        $id: GlobalID!
        $ck: UUID!
        $statementId: GlobalID!
        $statementCk: UUID!
        $statementKey: String!
        $value: JSON!
      ) {
        createRecord(
          input: {
            id: $id
            ck: $ck
            statementId: $statementId
            statementCk: $statementCk
            statementKey: $statementKey
            value: $value
          }
        ) {
          ... on Record {
            id
            ck
            createdAt
            updatedAt
            deletedAt
            revision
            value
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        id: string;
        ck: string;
        statementId: string;
        statementCk: string;
        statementKey: string;
        value: any;
      }) =>
        ({
          __typename: "Mutation",
          createRecord: {
            __typename: "Record",
            id: vars.id,
            ck: vars.ck,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            revision: PENDING_REVISION,
            value: vars.value,
          },
        } as CreateRecordMutation),
    }
  );

  const { mutate: updateRecordMut } = registry.defineEdit(
    EditType.UpdateRecord,
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

  const { mutate: deleteRecordMut } = registry.defineEdit(
    EditType.DeleteRecord,
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

  const { mutate: softDeleteRecordMut } = registry.defineEdit(
    EditType.SoftDeleteRecord,
    graphql(/* GraphQL */ `
      mutation softDeleteRecord($id: GlobalID!, $statementId: GlobalID!) {
        softDeleteRecord(input: { id: $id, statementId: $statementId }) {
          ... on Record {
            id
            deletedAt
            revision
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
            revision: PENDING_REVISION,
          },
        } as SoftDeleteRecordMutation),
    }
  );

  const { mutate: restoreRecordMut } = registry.defineEdit(
    EditType.RestoreRecord,
    graphql(/* GraphQL */ `
      mutation restoreRecord($id: GlobalID!, $statementId: GlobalID!) {
        restoreRecord(input: { id: $id, statementId: $statementId }) {
          ... on Record {
            id
            deletedAt
            revision
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
            // NOTE: revision is not actually needed here, but it seems the specific way we query for records in databases (with searchDatabase)
            // doesn't (always?) trigger reactivity through Apollo's cache properly if deletedAt is reset to null optimistically.
            // i.e. if we don't change something - like the pending revision - the record will still appear deleted on this client (only, it's just a local UX issue).
            revision: PENDING_REVISION,
          },
        } as RestoreRecordMutation),
    }
  );

  async function createRecord(
    tx: Transaction | null,
    id: string,
    ck: string,
    statementId: string,
    statementCk: string,
    statementKey: string,
    value: Scalars["JSON"]
  ) {
    await ops.perform({
      tx,
      type: "statement.createRecord",
      do: async () => {
        return await createRecordMut({
          id: id,
          ck: ck,
          statementId: statementId,
          statementCk: statementCk,
          statementKey: statementKey,
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

  const { mutate: createFieldMut } = registry.defineEdit(
    EditType.CreateField,
    graphql(/* GraphQL */ `
      mutation createField(
        $id: GlobalID!
        $ck: UUID!
        $statementId: GlobalID!
        $tag: TypeTag!
        $hint: TypeHint
        $key: String!
        $orderKey: String!
        $name: String
        $text: String
        $flags: Int!
        $referenceCk: UUID
        $value: JSON
      ) {
        createField(
          input: {
            id: $id
            ck: $ck
            statementId: $statementId
            tag: $tag
            hint: $hint
            key: $key
            orderKey: $orderKey
            name: $name
            text: $text
            flags: $flags
            referenceCk: $referenceCk
            value: $value
          }
        ) {
          ... on Field {
            # should match :FieldContent fragment
            id
            ck
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
            text
            referenceCk
            flags
            value
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
        ck: string;
        tag: string;
        hint: string | null;
        key: string;
        orderKey: string;
        statementId: string;
        name: string;
        text: string | null;
        flags: number;
        referenceCk: string | null;
        value: any;
      }) =>
        ({
          __typename: "Mutation",
          createField: {
            __typename: "Field",
            id: vars.id,
            ck: vars.ck,
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
            text: vars.text ?? null,
            orderKey: vars.orderKey,
            referenceCk: vars.referenceCk,
            flags: vars.flags,
            value: vars.value ?? null,
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

  const { mutate: deleteFieldMut } = registry.defineEdit(
    EditType.DeleteField,
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

  const { mutate: softDeleteFieldMut } = registry.defineEdit(
    EditType.SoftDeleteField,
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

  const { mutate: restoreFieldMut } = registry.defineEdit(
    EditType.RestoreField,
    graphql(/* GraphQL */ `
      mutation restoreField($id: GlobalID!) {
        restoreField(input: { id: $id }) {
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
          restoreField: {
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
      | "ck"
      | "key"
      | "name"
      | "text"
      | "tag"
      | "hint"
      | "orderKey"
      | "flags"
      | "value"
      | "referenceCk"
      | "statement"
    >
  ) {
    return {
      ...input,
      statementId,
      // set optional values to null if not provided
      hint: input.hint ?? null,
      text: input.text ?? null,
      referenceCk: input.referenceCk ?? null,
      flags: input.flags ?? 0,
    } as FieldCreateInput & { referenceCk: string | null; flags: number };
  }

  async function createField(
    tx: Transaction | null,
    statementId: string,
    field: Pick<
      Field,
      | "id"
      | "ck"
      | "name"
      | "tag"
      | "hint"
      | "text"
      | "key"
      | "orderKey"
      | "flags"
      | "value"
      | "referenceCk"
      | "statement"
    >
  ) {
    await ops.perform({
      tx,
      type: "statement.createField",
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
      type: "statement.deleteField",
      do: async () => {
        return await deleteFieldMut({ id: field.id });
      },
    });
  }

  async function softDeleteField(tx: Transaction | null, statementId: string, field: Pick<Field, "id">) {
    await ops.perform({
      tx,
      type: "statement.softDeleteField",
      do: async () => {
        return await softDeleteFieldMut({ id: field.id });
      },
      undo: async () => {
        return await restoreFieldMut({ id: field.id });
      },
    });
  }

  const { mutate: updateFieldMut } = registry.defineEdit(
    EditType.UpdateField,
    graphql(/* GraphQL */ `
      mutation updateField(
        $id: GlobalID!
        $tag: TypeTag!
        $hint: TypeHint
        $name: String
        $text: String
        $flags: Int!
        $referenceCk: UUID
        $value: JSON
      ) {
        updateField(
          input: {
            id: $id
            tag: $tag
            hint: $hint
            name: $name
            text: $text
            flags: $flags
            referenceCk: $referenceCk
            value: $value
          }
        ) {
          ... on Field {
            id
            tag
            hint
            updatedAt
            revision
            name
            text
            flags
            referenceCk
            value
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
        text: string;
        flags: number;
        referenceCk?: string;
        value?: any;
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
            text: vars.text,
            flags: vars.flags,
            referenceCk: vars.referenceCk ?? null,
            value: vars.value ?? null,
          },
        } as UpdateFieldMutation;
      },
    }
  );

  async function updateField(tx: Transaction | null, oldField: FieldUpdateInput, newField: FieldUpdateInput) {
    await ops.perform({
      tx,
      type: "statement.updateField",
      do: async () => {
        return await updateFieldMut(newField as any);
      },
      undo: async () => {
        return await updateFieldMut(oldField as any);
      },
    });
  }

  const { mutate: moveFieldMut } = registry.defineEdit(
    EditType.MoveField,
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
      type: "statement.moveField",
      do: async () => {
        return await moveFieldMut({ id, orderKey: newOrderKey });
      },
      undo: async () => {
        return await moveFieldMut({ id, orderKey: oldOrderKey });
      },
    });
  }

  const { mutate: createTaggingMut } = registry.defineEdit(
    EditType.CreateTagging,
    graphql(/* GraphQL */ `
      mutation createTagging(
        $id: GlobalID!
        $ck: UUID!
        $statementId: GlobalID!
        $key: String!
        $referenceCk: UUID!
        $value: JSON
      ) {
        createTagging(
          input: { id: $id, ck: $ck, statementId: $statementId, key: $key, referenceCk: $referenceCk, value: $value }
        ) {
          ... on Tagging {
            id
            ck
            revision
            key
            parent {
              id
            }
            referenceCk
            value
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
        ck: string;
        statementId: string;
        key: string;
        referenceCk: string | null;
        value: any;
      }) =>
        ({
          __typename: "Mutation",
          createTagging: {
            __typename: "Tagging",
            id: vars.id,
            ck: vars.ck,
            revision: PENDING_REVISION,
            key: vars.key,
            parent: {
              __typename: "Statement",
              id: vars.statementId,
            },
            referenceCk: vars.referenceCk,
            value: vars.value ?? null,
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

  const { mutate: deleteTaggingMut } = registry.defineEdit(
    EditType.DeleteTagging,
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

  const { mutate: softDeleteTaggingMut } = registry.defineEdit(
    EditType.SoftDeleteTagging,
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

  const { mutate: restoreTaggingMut } = registry.defineEdit(
    EditType.RestoreTagging,
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
    tagging: Pick<Tagging, "id" | "ck" | "key" | "referenceCk" | "value">
  ) {
    await ops.perform({
      tx,
      type: "statement.createTagging",
      do: async () => {
        return await createTaggingMut({
          id: tagging.id,
          ck: tagging.ck,
          statementId: statementId,
          key: tagging.key,
          referenceCk: tagging.referenceCk ?? null,
          value: tagging.value ?? null,
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
      type: "statement.deleteTagging",
      do: async () => {
        return await deleteTaggingMut({ id: tagging.id });
      },
    });
  }

  async function softDeleteTagging(tx: Transaction | null, statementId: string, tagging: Pick<Tagging, "id">) {
    await ops.perform({
      tx,
      type: "statement.softDeleteTagging",
      do: async () => {
        return await softDeleteTaggingMut({ id: tagging.id });
      },
      undo: async () => {
        return await restoreTaggingMut({ id: tagging.id });
      },
    });
  }

  const { mutate: updateTaggingMut } = registry.defineEdit(
    EditType.UpdateTagging,
    graphql(/* GraphQL */ `
      mutation updateTagging($id: GlobalID!, $value: JSON) {
        updateTagging(input: { id: $id, value: $value }) {
          ... on Tagging {
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
      optimisticResponse: (vars: { id: string; key: string; referenceCk: string | null; value: any }) =>
        ({
          updateTagging: {
            __typename: "Tagging",
            id: vars.id,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            value: vars.value ?? null,
          },
        } as any),
    }
  );

  function _toTaggingInput(input: Pick<Tagging, "id" | "key" | "referenceCk" | "value">) {
    return {
      ...input,
      // set optional values to null if not provided
      referenceCk: input.referenceCk,
    };
  }

  async function updateTaggingMetadata(
    tx: Transaction | null,
    statementId: string,
    oldTagging: Pick<Tagging, "id" | "value">,
    newTagging: Pick<Tagging, "id" | "value">
  ) {
    await ops.perform({
      tx,
      type: "statement.updateTaggingMetadata",
      do: async () => {
        return await updateTaggingMut(_toTaggingInput(newTagging as any) as any);
      },
      undo: async () => {
        return await updateTaggingMut(_toTaggingInput(oldTagging as any) as any);
      },
    });
  }

  const { mutate: createTriggerMut } = registry.defineEdit(
    EditType.CreateTrigger,
    graphql(/* GraphQL */ `
      mutation createTrigger(
        $id: GlobalID!
        $ck: UUID!
        $statementId: GlobalID!
        $type: TriggerType!
        $active: Boolean!
        $mapping: JSON
        $scheduleType: ScheduleType
        $timezone: String
        $interval: Int
        $cron: String
        $statementCk: UUID
        $scopeCk: UUID
      ) {
        createTrigger(
          input: {
            id: $id
            ck: $ck
            statementId: $statementId
            type: $type
            active: $active
            mapping: $mapping
            scheduleType: $scheduleType
            timezone: $timezone
            interval: $interval
            cron: $cron
            statementCk: $statementCk
            scopeCk: $scopeCk
          }
        ) {
          ... on Trigger {
            id
            ck
            parent {
              id
            }
            revision
            type
            active
            mapping
            scheduleType
            timezone
            interval
            cron
            statementCk
            scopeCk
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
        ck: string;
        statementId: string;
        type: TriggerType;
        active: boolean;
        mapping: any;
        scheduleType: ScheduleType | null;
        timezone: string | null;
        interval: number | null;
        cron: string | null;
        statementCk: string | null;
        scopeCk: string | null;
      }) =>
        ({
          __typename: "Mutation",
          createTrigger: {
            __typename: "Trigger",
            id: vars.id,
            ck: vars.ck,
            parent: {
              __typename: "Statement",
              id: vars.statementId,
            },
            revision: PENDING_REVISION,
            type: vars.type,
            active: vars.active,
            mapping: vars.mapping ?? null,
            scheduleType: vars.scheduleType ?? null,
            timezone: vars.timezone ?? null,
            interval: vars.interval ?? null,
            cron: vars.cron ?? null,
            statementCk: vars.statementCk,
            scopeCk: vars.scopeCk,
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
        const createTrigger = data?.createTrigger;
        if (createTrigger?.__typename != "Trigger") {
          return; // error
        }
        // extend Statement.triggers with (ref to) new trigger
        cache.modify({
          id: cache.identify(createTrigger.parent),
          fields: {
            triggers(existingTriggers = []) {
              const newRef = cache.identify(createTrigger);
              return [
                ...existingTriggers.filter((t: any) => t.__ref != newRef), // remove old trigger if exists
                { __ref: newRef },
              ];
            },
          },
          optimistic: true,
        });
      },
    }
  );

  const { mutate: softDeleteTriggerMut } = registry.defineEdit(
    EditType.SoftDeleteTrigger,
    graphql(/* GraphQL */ `
      mutation softDeleteTrigger($id: GlobalID!) {
        softDeleteTrigger(input: { id: $id }) {
          ... on Trigger {
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
          softDeleteTrigger: {
            __typename: "Trigger",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as any),
    }
  );

  const { mutate: restoreTriggerMut } = registry.defineEdit(
    EditType.RestoreTrigger,
    graphql(/* GraphQL */ `
      mutation restoreTrigger($id: GlobalID!) {
        restoreTrigger(input: { id: $id }) {
          ... on Trigger {
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
          restoreTrigger: {
            __typename: "Trigger",
            id: vars.id,
            deletedAt: null,
          },
        } as any),
    }
  );

  async function createTrigger(
    tx: Transaction | null,
    statementId: string,
    trigger: Pick<
      Trigger,
      | "id"
      | "ck"
      | "type"
      | "active"
      | "mapping"
      | "scheduleType"
      | "interval"
      | "timezone"
      | "cron"
      | "statementCk"
      | "scopeCk"
    >
  ) {
    await ops.perform({
      tx,
      type: "statement.createTrigger",
      do: async () => {
        return await createTriggerMut({
          id: trigger.id,
          ck: trigger.ck,
          statementId: statementId,
          type: trigger.type,
          active: trigger.active,
          mapping: trigger.mapping ?? null,
          scheduleType: trigger.scheduleType ?? null,
          timezone: trigger.timezone ?? null,
          interval: trigger.interval ?? null,
          cron: trigger.cron ?? null,
          statementCk: trigger.statementCk ?? null,
          scopeCk: trigger.scopeCk ?? null,
        });
      },
      undo: async () => {
        return await softDeleteTriggerMut({ id: trigger.id });
      },
      redo: async () => {
        return await restoreTriggerMut({ id: trigger.id });
      },
    });
  }

  async function softDeleteTrigger(tx: Transaction | null, statementId: string, trigger: Pick<Trigger, "id">) {
    await ops.perform({
      tx,
      type: "statement.softDeleteTrigger",
      do: async () => {
        return await softDeleteTriggerMut({ id: trigger.id });
      },
      undo: async () => {
        return await restoreTriggerMut({ id: trigger.id });
      },
    });
  }

  async function restoreTrigger(tx: Transaction | null, statementId: string, trigger: Pick<Trigger, "id">) {
    await ops.perform({
      tx,
      type: "statement.restoreTrigger",
      do: async () => {
        return await restoreTriggerMut({ id: trigger.id });
      },
      undo: async () => {
        return await softDeleteTriggerMut({ id: trigger.id });
      },
    });
  }

  const { mutate: updateTriggerMut } = registry.defineEdit(
    EditType.UpdateTrigger,
    graphql(/* GraphQL */ `
      mutation updateTrigger(
        $id: GlobalID!
        $type: TriggerType!
        $active: Boolean!
        $mapping: JSON
        $scheduleType: ScheduleType
        $timezone: String
        $interval: Int
        $cron: String
        $statementCk: UUID
        $scopeCk: UUID
      ) {
        updateTrigger(
          input: {
            id: $id
            type: $type
            active: $active
            mapping: $mapping
            scheduleType: $scheduleType
            timezone: $timezone
            interval: $interval
            cron: $cron
            statementCk: $statementCk
            scopeCk: $scopeCk
          }
        ) {
          ... on Trigger {
            id
            updatedAt
            type
            revision
            active
            mapping
            scheduleType
            timezone
            interval
            cron
            statementCk
            scopeCk
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        id: string;
        type: TriggerType;
        active: boolean;
        mapping: any;
        scheduleType: ScheduleType | null;
        timezone: string | null;
        interval: number | null;
        cron: string | null;
        statementCk: string | null;
        scopeCk: string | null;
      }) =>
        ({
          updateTrigger: {
            __typename: "Trigger",
            id: vars.id,
            updatedAt: new Date().toISOString(),
            revision: PENDING_REVISION,
            type: vars.type,
            active: vars.active,
            mapping: vars.mapping ?? null,
            scheduleType: vars.scheduleType ?? null,
            timezone: vars.timezone ?? null,
            interval: vars.interval ?? null,
            cron: vars.cron ?? null,
            statementCk: vars.statementCk ?? null,
            scopeCk: vars.scopeCk ?? null,
          },
        } as any),
    }
  );

  function _toTriggerInput(
    input: Pick<
      Trigger,
      | "id"
      | "type"
      | "active"
      | "scheduleType"
      | "mapping"
      | "timezone"
      | "interval"
      | "cron"
      | "statementCk"
      | "scopeCk"
    >
  ) {
    return {
      ...input,
      statementCk: input.statementCk ?? null,
      scopeCk: input.scopeCk ?? null,
    };
  }

  async function updateTrigger(
    tx: Transaction | null,
    oldTrigger: Pick<
      Trigger,
      | "id"
      | "type"
      | "active"
      | "mapping"
      | "scheduleType"
      | "timezone"
      | "interval"
      | "cron"
      | "statementCk"
      | "scopeCk"
    >,
    newTrigger: Pick<
      Trigger,
      | "id"
      | "type"
      | "active"
      | "mapping"
      | "scheduleType"
      | "timezone"
      | "interval"
      | "cron"
      | "statementCk"
      | "scopeCk"
    >
  ) {
    await ops.perform({
      tx,
      type: "statement.updateTrigger",
      do: async () => {
        return await updateTriggerMut(_toTriggerInput(newTrigger) as any);
      },
      undo: async () => {
        return await updateTriggerMut(_toTriggerInput(oldTrigger) as any);
      },
    });
  }

  return {
    registry,
    updateSymbolCode,
    updateStatementText,
    updateValue,
    createRecord,
    updateRecord,
    deleteRecord,
    softDeleteRecord,
    createField,
    updateField,
    moveField,
    deleteField,
    softDeleteField,
    createTagging,
    updateTaggingMetadata,
    deleteTagging,
    softDeleteTagging,
    createTrigger,
    softDeleteTrigger,
    restoreTrigger,
    updateTrigger,
  };
}
