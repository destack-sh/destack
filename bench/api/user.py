from rest_framework import serializers, viewsets

from bench.models.user import User


class UserSerializer(serializers.ModelSerializer):
    class Meta:
        model = User


class UserViewSet(viewsets.ModelViewSet):
    serializer_class = UserSerializer
