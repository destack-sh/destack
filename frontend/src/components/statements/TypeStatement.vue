<script lang="ts" setup>
import InlineActions from "@/components/statements/InlineActionsCell.vue";
import TypedDeclarationCell from "@/components/statements/TypedDeclarationCell.vue";
import { useElementRefs, useNavigationGrid } from "@/composables/useGrid";
import TypeTupleInterface from "@/components/interfaces/TypeTupleInterface.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { makeField, useStatementContext, type Field } from "@/state/statement";
import { TypeTag } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { newFieldId, newFieldKey } from "@/state/operations/statement";
import { TypeFlag } from "@/state/module";
import { generateKeyBetween } from "@/utils/fractional";
import { PlusIcon, SquaresPlusIcon } from "@heroicons/vue/24/outline";
import CubeTransparentIcon from "@heroicons/vue/24/outline/CubeTransparentIcon";
import { computed, nextTick, ref, type Ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof TypedDeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);

const isEnum = computed(() => context.typeRootTag.value == TypeTag.Enum);
const selfFields = computed(
  () => context.fields.value.filter((n) => !(n.flags & TypeFlag.IsUnionWith)).map((n) => n as Field) ?? []
);
const fieldsLength = computed(() => selfFields.value?.length ?? 0);
const baseTypes = computed(
  () => context.fields.value.filter((n) => n.flags & TypeFlag.IsUnionWith).map((n) => n as Field) ?? []
);
const addMemberRef: Ref<HTMLButtonElement | null> = ref(null);
const addingDescription = ref(false);

// dynamic member refs for names, values & descriptions for each member
type ColumnType = "type";
const grid = useNavigationGrid<ColumnType, InstanceType<typeof TypeTupleInterface>>(
  ref(["type"] as ColumnType[]),
  selfFields,
  {
    gridNavigateUp: focusDescriptionFromBottom,
    gridNavigateDown,
  }
);
const isEditing = computed(() => grid.refs.value.find((n) => n.editing));

// TODO @Cleanup: reduce duplication between type definition & data definition cells (also in view below)
// perhaps move into statement context?

function insertMember(isUnionWith?: boolean) {
  const lastMember = context.fields.value?.[context.fields.value.length - 1];
  const orderKey = generateKeyBetween(lastMember?.orderKey ?? null, null);
  if (isEnum.value) {
    // enum member
    const name = "Option " + (fieldsLength.value + 1);
    const newMemberNode = makeField({
      name,
      tag: TypeTag.Literal,
      value: name, // :LiteralStringEnum
      orderKey,
    });
    context.createField(newMemberNode);
    nextTick(() => grid.focus(newMemberNode.id, "type"));
  } else if (!isUnionWith) {
    // struct field
    const newMemberNode = makeField({
      name: "field " + (fieldsLength.value + 1),
      tag: TypeTag.String,
      orderKey,
      flags: TypeFlag.IsNullable, // :DefaultTypeFlags
    });
    context.createField(newMemberNode);
    nextTick(() => grid.focus(newMemberNode.id, "type"));
  } else {
    const newMemberNode = makeField({
      name: "",
      tag: TypeTag.TypeReference,
      orderKey,
      flags: TypeFlag.IsUnionWith,
    });
    context.createField(newMemberNode);
    nextTick(() => declarationRef.value?.focusLastBase());
  }
}

function duplicateMember(memberId: string) {
  // :DuplicateField
  const memberIdx = selfFields.value?.findIndex((m) => m.id === memberId);
  if (memberIdx < 0) return;
  const member = selfFields.value?.[memberIdx];
  const orderKey = generateKeyBetween(member?.orderKey ?? null, selfFields.value?.[memberIdx + 1]?.orderKey ?? null);
  // "name" => "name 2", "name 2" => "name 3", etc.
  const newName =
    member.name?.replace(/(\d+)?$/, (_, num) => (parseInt(num ?? "1") + 1).toString()) ?? member.name + " 2";
  const newMemberNode = {
    ...member,
    id: newFieldId(),
    key: newFieldKey(),
    name: newName,
    orderKey,
  };
  context.createField(newMemberNode);
  nextTick(() => grid.focus(memberIdx + 1, "type"));
}

function deleteMember(memberId: string) {
  const memberIdx = selfFields.value?.findIndex((m) => m.id === memberId);
  if (memberIdx == null || memberIdx < 0) {
    return;
  }
  const member = selfFields.value?.[memberIdx];
  context.deleteField(member as any); // must exist
  grid.focus(memberIdx - 1, "type"); // move focus above
}

function moveMember(node: Field, position: "before" | "after", other: Field) {
  const otherIndex = selfFields.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(selfFields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    context.moveField(node, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, selfFields.value[otherIndex + 1]?.orderKey ?? null);
    context.moveField(node, orderKey);
  }
}

function dropMember(droppedId: string, position: "above" | "below", memberId: string) {
  const dropped = selfFields.value.find((n) => n.id == droppedId);
  const member = selfFields.value.find((n) => n.id == memberId);
  if (dropped == null || member == null || dropped.id == member.id) return; // ignore invalid / cross statement drops
  moveMember(dropped, ["above", "left"].includes(position) ? "before" : "after", member);
  nextTick(() => grid.focus(droppedId, "type"));
}

function writeType(memberId: string, newType: Field) {
  const oldType = selfFields.value?.find((m) => m.id === memberId);
  if (!oldType) return;
  context.updateField(oldType, { ...oldType, ...newType, id: memberId });
}

function writeDescription(memberId: string, newDescription: string) {
  const oldType = selfFields.value?.find((m) => m.id === memberId);
  if (!oldType) return;
  context.updateField(oldType, { ...oldType, description: newDescription });
}

function focusDescriptionFromTop() {
  if (description.value?.length > 0 || addingDescription.value) {
    descriptionRef.value?.focus();
  } else {
    focusFirstIfExists();
  }
}

function focusDescriptionFromBottom() {
  if (description.value?.length > 0 || addingDescription.value) {
    descriptionRef.value?.focus();
  } else {
    declarationRef.value?.focus();
  }
}

function focusFirstIfExists() {
  if (fieldsLength.value == 0) {
    addMemberRef.value?.focus();
  } else {
    grid.focus(0, "type");
  }
}

function focusLast() {
  if (fieldsLength.value > 0) {
    grid.focus(fieldsLength.value - 1, "type");
  } else {
    focusDescriptionFromBottom();
  }
}

function gridNavigateDown() {
  addMemberRef.value?.focus();
}

const extraInlineActions = computed(() => {
  const inlineActions: StatementAction[] = [];
  inlineActions.push({
    label: "Add " + (isEnum.value ? "option" : "field"),
    icon: SquaresPlusIcon,
    action: () => insertMember(),
  });
  if (!isEnum.value) {
    inlineActions.push({
      label: "Extend",
      icon: CubeTransparentIcon,
      action: () => insertMember(true),
    });
  }
  return inlineActions;
});

defineExpose({
  focus: (position: "first" | "last" = "first") =>
    position == "first" ? declarationRef.value?.focus() : addMemberRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
    addMemberRef.value?.blur();
    grid.blur();
  },
});
</script>
<template>
  <!-- Declaration -->
  <div class="flex flex-row justify-between">
    <div class="flex flex-row items-baseline">
      <TypedDeclarationCell
        ref="declarationRef"
        @navigate-down="focusDescriptionFromTop"
        @navigate-up="context.navigateUp"
      />
    </div>
    <InlineActions
      class="transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
      :extraActions="extraInlineActions"
    />
  </div>
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    :class="addingDescription ? '' : 'h-0'"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-left="declarationRef?.focus()"
    @navigate-up="declarationRef?.focus()"
    @navigate-down="focusFirstIfExists"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0 && !context.readonly.value && addingDescription"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
  >
    +description
  </button>
  <!-- Members (enum options or struct fields) -->
  <div v-if="fieldsLength > 0" class="my-0.5 flex w-full flex-col gap-0.5">
    <TypeTupleInterface
      v-for="member of selfFields"
      :key="member.id"
      :model-value="member"
      @update:model-value="(val: any) => writeType(member.id, val)"
      :ref="(el: any) => grid.registerColumnRef(member.id, 'type', el)"
      :readonly="context.readonly.value"
      :isEnum="isEnum"
      :tupleName="isEnum ? 'option' : 'field'"
      orientation="vertical"
      @navigate-left="grid.navigateLeft(member.id, 'type')"
      @navigate-right="grid.navigateRight(member.id, 'type')"
      @navigate-up="grid.navigateUp(member.id, 'type')"
      @navigate-down="grid.navigateDown(member.id, 'type')"
      @delete-self="deleteMember(member.id)"
      @duplicate-self="duplicateMember(member.id)"
      @keydown.delete.exact="isEditing || deleteMember(member.id)"
      @drop="(p, v) => dropMember(v.id, p, member.id)"
      @enter="grid.navigateDown(member.id, 'type')"
      class="-mx-1 self-start px-1 py-0.5 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
      :class="isEnum ? 'w-fit' : 'w-full '"
    />
  </div>
  <div class="mb-1">
    <!-- Add a member -->
    <button
      v-show="!context.readonly.value"
      tabindex="-1"
      ref="addMemberRef"
      class="mt-1 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="insertMember()"
      @enter="insertMember()"
      @keydown.up.exact.prevent="focusLast"
      @keydown.down.exact.prevent="context.navigateDown"
    >
      <PlusIcon class="h-4 w-4" />{{ isEnum ? "Option" : "Field" }}
    </button>
  </div>
</template>
