<script lang="ts" setup>
import { TypeTag, type Field, StatementType } from "@/gql/graphql";
import { TypeFlag, useCurrentModule, useNavigation } from "@/state/module";
import { STATEMENT_TYPE_TAGS, getStatementIconOutline } from "@/state/statement";
import { ICONS_BY_HINT_OUTLINE, ICONS_BY_TAG_OUTLINE, renderBuiltinType } from "@/state/type";
import { ListBulletIcon } from "@heroicons/vue/24/outline";
import { useKeyModifier } from "@vueuse/core";
import { computed } from "vue";

const props = defineProps<{
  type: Field;
  showTypeName?: boolean;
  hideIcon?: boolean;
  hideFlags?: boolean;
  hideReference?: boolean;
}>();

const module = useCurrentModule();
const resolvedReference = computed(() => {
  if (props.type.referenceCk != null) {
    return module.statementOf(props.type.referenceCk);
  } else {
    return null;
  }
});
const nav = useNavigation();
const altState = useKeyModifier("Alt");

const resolvedTag = computed(
  () => STATEMENT_TYPE_TAGS[resolvedReference.value?.type as StatementType] ?? props.type.tag
);

const icon = computed(() => {
  if (
    resolvedReference.value != null &&
    [StatementType.Code, StatementType.Flow, StatementType.Task, StatementType.Database].includes(
      resolvedReference.value?.type
    )
  ) {
    return getStatementIconOutline(resolvedReference.value?.type);
  } else if (props.type.hint != null && ICONS_BY_HINT_OUTLINE[props.type.hint] != null) {
    return ICONS_BY_HINT_OUTLINE[props.type.hint];
  } else if (ICONS_BY_TAG_OUTLINE[resolvedTag.value] != null) {
    return ICONS_BY_TAG_OUTLINE[resolvedTag.value];
  } else {
    return null;
  }
});
const showName = computed(() => (!icon.value || props.showTypeName) && props.type.referenceCk == null);
</script>
<template>
  <div class="relative whitespace-nowrap">
    <!-- Force icon to align with text -->
    <!-- works fine but there has to be a better way... -->
    <div v-if="icon && !hideIcon" class="inline-block h-4 w-4">
      &nbsp;
      <component :is="icon" class="absolute left-0 top-0.5 h-4 w-4" :class="showTypeName ? 'top-0.5' : 'top-0'" />
    </div>
    <span v-if="showName" class="ml-1">
      {{ renderBuiltinType(resolvedTag, type.hint ?? null) }}
    </span>
    <span
      v-if="(resolvedTag == TypeTag.TypeReference || type.referenceCk) && !hideReference"
      class="ml-1 mr-1"
      :class="[altState ? 'decoration-gray-500 underline-offset-4 hover:underline' : '']"
      @click="
        (e) => {
          if (altState && resolvedReference != null) {
            e.stopPropagation();
            e.preventDefault();
            nav.focusStatement(resolvedReference);
          }
        }
      "
    >
      {{ resolvedReference?.name ?? "???" }}
    </span>
    <!-- TODO @UX: show required type flag -->
    <!-- Flags -->
    <div v-if="type.flags & TypeFlag.IsArray && !hideFlags" class="relative left-1 mr-1 inline-block h-4 w-4">
      <ListBulletIcon class="absolute left-0 top-0.5 h-4 w-4" />
    </div>
  </div>
</template>
