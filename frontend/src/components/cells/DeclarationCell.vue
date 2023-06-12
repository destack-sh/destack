<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import ModifierCell from "@/components/cells/ModifierCell.vue";
import SelectTypeInterface from "@/components/cells/ProtoStatementTypeCell.vue";
import StatementTypeCell from "@/components/cells/StatementTypeCell.vue";
import { useStatementContext } from "@/state/statement";
import { StatementType } from "@/gql/graphql";
import { computed, ref, type Ref } from "vue";
import { useCurrentModule } from "@/state/module";

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
const gapRef: Ref<InstanceType<typeof SelectTypeInterface> | null> = ref(null);

const module = useCurrentModule();
const availableSymbols = module.statementsLike({
  types: [context.statement.value?.type],
  includeDependencies: true,
});

function deleteModifierOrAbove() {
  if (context.statement.value.modifier != null) {
    context.setModifier(null);
  } else {
    context.tryDeleteLeft();
  }
}

// runtime
const localErrors = module.localErrorsOf(context.statement);

defineExpose({
  focus: (position: "first" | "last" = "first") => nameRef.value?.focus(),
  blur: () => {
    startRef.value?.blur();
    nameRef.value?.blur();
    gapRef.value?.blur();
  },
});
</script>
<template>
  <div class="relative flex w-fit flex-row items-baseline gap-1 whitespace-nowrap">
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
    <SelectTypeInterface
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
    <StatementTypeCell />
    <!-- Name or ref -->
    <EditableSpan
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
    <button
      tabindex="-1"
      v-if="!hasName && context.statement.value.type == StatementType.Symbol"
      @click="nameRef?.focus()"
      class="-ml-1 w-fit select-none rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
    >
      +name
    </button>
    <!-- Error underline for declaration if unlocated -->
    <div
      v-if="localErrors != null && localErrors.length > 0"
      class="absolute bottom-0 left-0 h-0.5 w-full bg-red-600"
    />
  </div>
</template>
