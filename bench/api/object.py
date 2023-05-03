from typing import Optional
from uuid import UUID

from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import check_can_write_project
from bench.api.util import asafe_mutation


@gql.django.type(models.RemoteObject)
class RemoteObject:
    id: UUID
    md5: str
    content_length: int
    content_type: str
    name: Optional[str]
    presigned_post: Optional[str]


@gql.input
class RequestUploadObjectInput:
    project_id: UUID
    md5: str
    content_length: int
    content_type: str
    name: Optional[str]


@gql.input
class NotifyUploadedObjectInput(gql.NodeInput):
    pass


@gql.input
class DeleteObjectInput(gql.NodeInput):
    pass


@gql.type
class ObjectMutation:
    @asafe_mutation
    def request_upload_object(
        self, info: Info, input: RequestUploadObjectInput
    ) -> RemoteObject | OperationInfo:
        project = models.Project.objects.get(input.project_id)
        check_can_write_project(info, project)

    @asafe_mutation
    def notify_uploaded_object(
        self, info: Info, input: NotifyUploadedObjectInput
    ) -> RemoteObject | OperationInfo:
        remote_object = models.RemoteObject.objects.get(id=input.id)
        check_can_write_project(info, remote_object.project)
        # TODO @Robustness: check in aws
        remote_object.status = models.RemoteObjectStatus.AVAILABLE
        remote_object.save()
        return remote_object

    @asafe_mutation
    def delete_object(self, info: Info, input: DeleteObjectInput) -> RemoteObject | OperationInfo:
        object = models.RemoteObject.objects.get(input.id)
        check_can_write_project(info, object.project)
        object.delete()
        return object
