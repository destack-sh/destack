from rest_framework import serializers

from bench.models.task import Task


class TaskSerializer(serializers.ModelSerializer):
    class Meta:
        model = Task
