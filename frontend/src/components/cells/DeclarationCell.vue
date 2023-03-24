<script lang="ts" setup>
import ModifierCell from "@/components/cells/ModifierCell.vue";
import ReferenceComboCell from "@/components/cells/ReferenceComboCell.vue";
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import SymbolTypeCell from "@/components/cells/SymbolTypeCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { StatementType } from "@/gql/graphql";
import { localErrorsOf, symbolsLike } from "@/state/runtime";
import { computed, ref, type Ref } from "vue";

const context = useStatementContext();

const emit = defineEmits<{
  (e: "navigateDown"): void;
  (e: "navigateRight"): void;
}>();

const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const name: Ref<string> = ref(context.statement.value.name ?? "");
context.syncName(
  name,
  computed(() => nameRef.value?.focused)
);
const hasName = computed(() => name.value.trim().length > 0);
const startRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const gapRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);

const availableSymbols = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: context.statement.value.symbolType != null ? [context.statement.value?.symbolType] : undefined,
  includeDependencies: true,
});

function deleteModifierOrAbove() {
  if (context.statement.value.modifier != null) {
    context.setModifier(null);
  } else {
    context.tryDeleteLeft();
  }
}

const localErrors = localErrorsOf(context.statement);

defineExpose({
  focus: () => nameRef.value?.focus(),
  blur: () => {
    startRef.value?.blur();
    nameRef.value?.blur();
    gapRef.value?.blur();
  },
});
</script>
<template>
  <div class="relative flex w-fit flex-row flex-wrap items-baseline gap-1">
    <!-- Start trap -->
    <EditableSpan
      :model-value="''"
      ref="startRef"
      class="-mx-0.5"
      v-if="context.statement.value.modifier != null"
      @navigate-up="context.navigateUp"
      @navigate-down="emit('navigateDown')"
      @navigate-right="gapRef?.focus()"
      @delete-left="context.tryDeleteLeft"
      @enter="context.insertAbove"
      @escape="context.escape"
      :readonly="context.readonly.value"
    />
    <!-- Modifier -->
    <ModifierCell v-if="context.statement.value.modifier" />
    <SelectTypeCell
      class="-mx-0.5"
      ref="gapRef"
      @navigate-up="context.navigateUp"
      @navigate-down="emit('navigateDown')"
      @navigate-left="startRef?.focus()"
      @navigate-right="nameRef?.focus()"
      @delete-left="deleteModifierOrAbove"
      @enter="context.insertAbove"
      @escape="context.escape"
    />
    <SymbolTypeCell />
    <!-- Name or ref -->
    <EditableSpan
      v-if="context.statement.value.type == StatementType.Definition"
      ref="nameRef"
      class="mx-0.5"
      v-model="name"
      :readonly="context.readonly.value"
      @navigate-up="context.navigateUp"
      @navigate-down="emit('navigateDown')"
      @navigate-left="gapRef?.focus"
      @navigate-right="emit('navigateRight')"
      @escape="context.escape"
      @enter="context.insertBelow"
    />
    <ReferenceComboCell
      v-else-if="context.statement.value.type == StatementType.Reference"
      ref="nameRef"
      :self="context.statement.value"
      :reference="context.reference.value"
      :available-symbols="availableSymbols"
      :readonly="context.readonly.value"
      @navigate-up="context.navigateUp"
      @navigate-down="emit('navigateDown')"
      @navigate-left="gapRef?.focus"
      @navigate-right="emit('navigateRight')"
      @insert-below="context.insertBelow"
      @escape="context.escape"
      @enter="context.insertBelow"
      @set-reference="context.setReference"
    />
    <button
      tabindex="-1"
      v-if="!hasName && context.statement.value.type == StatementType.Definition"
      @click="nameRef?.focus()"
      class="-ml-1 w-fit select-none rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
    >
      +name
    </button>
    <!-- Indicate specific kind of type definition (e.g. choice if a type) -->
    <span class="ml-1 text-orange-600" :contenteditable="false" v-if="context.symbolSubtype.value != null">
      {{ context.symbolSubtype.value }}
    </span>
    <!-- Error underline for declaration if unlocated -->
    <div
      v-if="localErrors != null && localErrors.length > 0"
      class="absolute bottom-0 left-0 h-0.5 w-full bg-red-600"
    />
  </div>
</template>
