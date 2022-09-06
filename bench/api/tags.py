from typing import Optional

from rest_framework import serializers, viewsets

from bench.models import Tag
from bench.models.tag import TaggedItem


class TagSerializer(serializers.ModelSerializer):
    class Meta:
        model = Tag
        fields = ["name", "description", "created_at", "updated_at", "metadata"]


class TaggedItemSerializerMixin(serializers.Serializer):
    tags = serializers.ListField(required=False)

    def to_representation(self, obj):
        result = super(TaggedItemSerializerMixin, self).to_representation(obj)
        if hasattr(obj, "prefetched_tags"):
            result["tags"] = [t.tag.name for t in obj.prefetched_tags]
        else:
            result["tags"] = obj.tagged_items.values_list("tag__name", flat=True)
        return result

    def _set_tags(self, obj, tags: Optional[list[str]]):
        if obj is None or tags is None:
            return

        current_tags = set(tags)  # deduplicate
        # create/set new tags
        tagged_item_instances = []
        for tag in current_tags:
            tag_instance, _ = Tag.objects.get_or_create(name=tag)
            tagged_item_instance, _ = obj.tagged_items.get_or_create(tag_id=tag_instance.id)
            tagged_item_instances.append(tagged_item_instance)
        # delete extraneous tagged
        obj.tagged_items.exclude(tag__name__in=current_tags).delete()

        obj.prefetched_tags = tagged_item_instances

    def create(self, validated_data):
        validated_data.pop("tags", None)
        instance = super(TaggedItemSerializerMixin, self).create(validated_data)
        self._set_tags(instance, self.initial_data.get("tags"))
        return instance

    def update(self, instance, validated_data):
        instance = super(TaggedItemSerializerMixin, self).update(instance, validated_data)
        self._set_tags(instance, self.initial_data.get("tags"))
        return instance


class TagViewSet(viewsets.ModelViewSet):
    queryset = Tag.objects.all()
    serializer_class = TagSerializer
    lookup_field = "name"


class TaggedItemViewSetMixin(viewsets.GenericViewSet):
    def get_queryset(self):
        queryset = super(TaggedItemViewSetMixin, self).get_queryset()
        return queryset.prefetch_related(
            "tagged_items",
            queryset=TaggedItem.objects.select_related("tag"),
            to_attr="prefetched_tags",
        )
