<script lang="ts" setup>
import { TypeTag, type SimpleType } from "@/gql/graphql";
import { TYPETAG_KEYWORD } from "@/state/editor";
import { symbolOf } from "@/state/runtime";
import {
  ArrowDownCircleIcon,
  ArrowUpRightIcon,
  Bars3BottomLeftIcon,
  CheckIcon,
  DocumentIcon,
  HashtagIcon,
  MinusSmallIcon,
  PhotoIcon,
  SparklesIcon,
  SpeakerWaveIcon,
  Squares2X2Icon,
  VideoCameraIcon,
} from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{ type: SimpleType; showTypeName?: boolean; hideIcon?: boolean; hideReference?: boolean }>();

// resolve type since the type reference is likely not included
// (this hack will be removed once the special interp state finally dies)
const resolvedReference = computed(() => {
  if (props.type.reference == null) {
    return;
  } else if (props.type.reference.name != null) {
    return props.type.reference;
  } else {
    return symbolOf(props.type.reference.id);
  }
});

const resolvedTag = computed(() => resolvedReference.value?.rootTypeTag ?? props.type.tag);

const iconsByTag: Record<TypeTag, any> = {
  [TypeTag.String]: Bars3BottomLeftIcon,
  [TypeTag.Number]: HashtagIcon,
  [TypeTag.Boolean]: CheckIcon,
  [TypeTag.Embedding]: SparklesIcon,
  [TypeTag.Null]: MinusSmallIcon,
  [TypeTag.File]: DocumentIcon,
  [TypeTag.Image]: PhotoIcon,
  [TypeTag.Audio]: SpeakerWaveIcon,
  [TypeTag.Video]: VideoCameraIcon,
  [TypeTag.TypeReference]: ArrowUpRightIcon,
  [TypeTag.Struct]: Squares2X2Icon,
  [TypeTag.Enum]: ArrowDownCircleIcon,
};
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-1.5">
    <!-- Force icon to align with text -->
    <!-- works fine but there has to be a better way... -->
    <span v-if="iconsByTag[resolvedTag] && !hideIcon" class="relative h-4 w-4">
      <span class="opacity-0">t</span>
      <component
        :is="iconsByTag[resolvedTag]"
        class="absolute left-0 h-4 w-4"
        :class="showTypeName ? 'top-0.5' : 'top-0'"
      />
    </span>
    <span v-if="!iconsByTag[resolvedTag] || (showTypeName && type.reference == null)">{{
      TYPETAG_KEYWORD[resolvedTag]
    }}</span>
    <span v-if="type.reference && !hideReference">{{ resolvedReference?.name ?? "???" }}</span>
    <span v-else-if="type.tag == TypeTag.TypeReference && type.reference == null">...</span>
  </div>
</template>
