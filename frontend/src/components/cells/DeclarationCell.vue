<script lang="ts" setup>
import ModifierCell from "@/components/cells/ModifierCell.vue";
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import SymbolTypeCell from "@/components/cells/SymbolTypeCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";

const context = useStatementContext();

const emit = defineEmits<{
  (e: "navigateDown"): void;
  (e: "navigateRight"): void;
}>();

const name: Ref<string> = ref(context.statement.value.name ?? "");
context.syncName(name);
const startRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const gapRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

function deleteModifierOrAbove() {
  if (context.statement.value.modifier != null) {
    context.setModifier(null);
  } else {
    context.tryDeleteLeft();
  }
}

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
  <div class="flex flex-row gap-1">
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
      @delete-right="context.setModifier(null)"
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
  </div>
</template>
