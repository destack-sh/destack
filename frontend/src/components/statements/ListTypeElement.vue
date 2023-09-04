<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import InlineActions from "@/components/statements/StatementActions.vue";
import TypedStatementDeclaration from "@/components/statements/TypedStatementDeclaration.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { TypeTag } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { makeField, useStatementContext } from "@/state/statement";
import { generateKeyBetween } from "@/utils/fractional";
import { Bars3Icon, PlusIcon, SquaresPlusIcon, TagIcon } from "@heroicons/vue/24/outline";
import CubeTransparentIcon from "@heroicons/vue/24/outline/CubeTransparentIcon";
import { computed, nextTick, ref, type Ref } from "vue";
import StatementTags from "@/components/statements/StatementTags.vue";
import { useCurrentModule, type Field } from "@/state/module";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void; (e: "toggleActions"): void }>();

const context = useStatementContext();
const module = useCurrentModule();
const declarationRef: Ref<InstanceType<typeof TypedStatementDeclaration> | null> = ref(null);
const tagsRef: Ref<InstanceType<typeof StatementTags> | null> = ref(null);
const text: Ref<string> = ref(context.statement.value.text ?? "");
const textRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncText(
  text,
  computed(() => textRef.value?.focused)
);

const isEnum = computed(() => context.typeRootTag.value == TypeTag.Enum);
const fieldsLength = computed(() => context.selfFields.value?.length ?? 0);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const addingText = ref(false);
const showText = computed(() => text.value.length > 0 || addingText.value);

// dynamic field refs for names, values & texts for each field
type ColumnType = "type";
const grid = useNavigationGrid<ColumnType, InstanceType<typeof FieldInterface>>(
  ref(["type"] as ColumnType[]),
  context.selfFields,
  {
    gridNavigateUp: focusTextFromBottom,
    gridNavigateDown,
  }
);
const isEditing = computed(() => grid.refs.value.find((n) => n.editing));

function createOption() {
  const lastField = context.fields.value?.[context.fields.value.length - 1];
  const orderKey = generateKeyBetween(lastField?.orderKey ?? null, null);
  const name = "Option " + (fieldsLength.value + 1);
  const newField = makeField({
    projectVersionId: module.id.value,
    name,
    tag: TypeTag.Literal,
    orderKey,
  });
  context.createField(newField);
  nextTick(() => {
    grid.getRef(newField.id, "type").open("all");
  });
}

function createUnionField() {
  context.createUnionField();
  nextTick(() => declarationRef.value?.focusLastBase());
}

function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "metadata"> & Partial<Field>) {
  const field = context.createNewField(template);
  nextTick(() => {
    grid.getRef(field.id, "type").open("all");
  });
}

function duplicateField(fieldId: string) {
  const newField = context.duplicateField(fieldId);
  if (newField != null) {
    nextTick(() => {
      grid.getRef(newField?.id, "type").open("all");
    });
  }
}

function deleteField(fieldId: string) {
  const fieldIdx = context.selfFields.value?.findIndex((m) => m.id === fieldId);
  if (fieldIdx == null || fieldIdx < 0) {
    return;
  }
  const field = context.selfFields.value?.[fieldIdx];
  context.deleteField(field as Field); // must exist
  grid.focus(fieldIdx - 1, "type"); // move focus above
}

function moveField(node: Field, position: "before" | "after", other: Field) {
  const otherIndex = context.selfFields.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(context.selfFields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    context.moveField(node, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, context.selfFields.value[otherIndex + 1]?.orderKey ?? null);
    context.moveField(node, orderKey);
  }
}

function dropField(droppedId: string, position: "above" | "below" | "left" | "right", fieldId: string) {
  const dropped = context.selfFields.value.find((n) => n.id == droppedId);
  const field = context.selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  moveField(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
  nextTick(() => grid.focus(droppedId, "type"));
}

function writeType(fieldId: string, newType: Field) {
  const oldType = context.selfFields.value?.find((m) => m.id === fieldId);
  if (!oldType) return;
  context.updateField(oldType, { ...oldType, ...newType, id: fieldId });
}

function focusTextFromTop() {
  if (props.folded) {
    context.navigateDown();
  } else if (text.value?.length > 0 || addingText.value) {
    textRef.value?.focus();
  } else {
    focusFirstIfExists();
  }
}

function focusTextFromBottom() {
  if (text.value?.length > 0 || addingText.value) {
    textRef.value?.focus();
  } else {
    declarationRef.value?.focus();
  }
}

function focusFirstIfExists() {
  if (fieldsLength.value == 0) {
    addFieldRef.value?.focus();
  } else {
    grid.focus(0, "type");
  }
}

function focusLast() {
  if (fieldsLength.value > 0) {
    grid.focus(fieldsLength.value - 1, "type");
  } else {
    focusTextFromBottom();
  }
}

function gridNavigateDown() {
  addFieldRef.value?.focus();
}

function unfoldIfFolded() {
  if (props.folded) emit("toggleFold");
}

const extraActions = computed(() => {
  const actions: StatementAction[] = [
    {
      label: "Add tag",
      icon: TagIcon,
      action: () => {
        unfoldIfFolded();
        tagsRef.value?.open();
      },
    },
    {
      label: "Add text",
      icon: Bars3Icon,
      disabled: showText.value,
      action: () => {
        unfoldIfFolded();
        addingText.value = true;
        nextTick(() => textRef.value?.focus());
      },
    },
  ];
  actions.push({
    label: "Add " + (isEnum.value ? "option" : "field"),
    icon: SquaresPlusIcon,
    action: () => (unfoldIfFolded(), isEnum.value ? createOption() : createFieldRef.value?.show()),
  });
  if (!isEnum.value) {
    actions.push({
      label: "Extend type",
      icon: CubeTransparentIcon,
      action: () => (unfoldIfFolded(), createUnionField()),
      hideInline: true,
    });
  }
  return actions;
});
context.setCustomActions(extraActions);

defineExpose({
  focus: (position: "first" | "last" = "first") =>
    position == "first" || props.folded ? declarationRef.value?.focus() : addFieldRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    textRef.value?.blur();
    addFieldRef.value?.blur();
    grid.blur();
  },
});
</script>
<template>
  <!-- Fields (enum options or struct fields) -->
  <div v-if="fieldsLength > 0 && !folded" class="mb-0.5 flex w-full flex-col">
    <FieldInterface
      v-for="field of context.selfFields.value"
      :key="field.id"
      :model-value="field"
      @update:model-value="(val: any) => writeType(field.id, val)"
      :ref="(el: any) => grid.registerColumnRef(field.id, 'type', el)"
      :readonly="context.readonly.value"
      :isEnum="isEnum"
      :tupleName="isEnum ? 'option' : 'field'"
      orientation="vertical"
      @navigate-left="grid.navigateLeft(field.id, 'type')"
      @navigate-right="grid.navigateRight(field.id, 'type')"
      @navigate-up="grid.navigateUp(field.id, 'type')"
      @navigate-down="grid.navigateDown(field.id, 'type')"
      @delete-self="deleteField(field.id)"
      @duplicate-self="duplicateField(field.id)"
      @keydown.delete.exact="isEditing || deleteField(field.id)"
      @drop="(p, v) => dropField(v.id, p, field.id)"
      @enter="grid.navigateDown(field.id, 'type')"
      class="-mx-1 self-start px-1 py-1 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
    />
  </div>
  <div class="mb-1">
    <!-- Add a field -->
    <button
      v-show="!context.readonly.value && !folded"
      tabindex="-1"
      ref="addFieldRef"
      class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="isEnum ? createOption() : createFieldRef?.show()"
      @enter="isEnum ? createOption() : createFieldRef?.show()"
      @keydown.up.exact.prevent="focusLast"
      @keydown.down.exact.prevent="context.navigateDown"
    >
      <PlusIcon class="h-4 w-4" />{{ isEnum ? "Option" : "Field" }}
    </button>
    <!-- Create popup right below button -->
    <CreateFieldInterface
      ref="createFieldRef"
      :title="'New field on ' + context.statement.value.name"
      @select="createNewField"
    />
  </div>
</template>
