from typing import Optional

import strawberry
import strawberry_django
import structlog
from django.core.exceptions import ValidationError
from strawberry import relay
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_module_access
from bench.api.utils import safe_mutation
from bench.language.remote import REMOTE_OBJECT_MAX_SIZE
from bench.models import ModuleAccessLevel
from bench.models.object import is_allowed_content_type

logger = structlog.get_logger(__name__)

RemoteObjectStatus = strawberry.enum(models.RemoteObjectStatus)


@strawberry_django.type(models.RemoteObject)
class RemoteObject(relay.Node):
    status: RemoteObjectStatus
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]
    presigned_post: Optional[str]
    presigned_get: Optional[str]


@strawberry.input
class RequestUploadObjectInput:
    project_id: GlobalID
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]


@strawberry.input
class NotifyUploadedObjectInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class DeleteObjectInput(strawberry_django.NodeInput):
    pass


@strawberry.type
class ObjectMutation:
    @safe_mutation
    def request_upload_object(
        self, info: Info, input: RequestUploadObjectInput
    ) -> RemoteObject | OperationInfo:
        project = models.Project.objects.get(id=input.project_id.node_id)
        check_module_access(info, project, ModuleAccessLevel.Read)
        if input.content_length >= REMOTE_OBJECT_MAX_SIZE:
            raise ValidationError(
                f"object too large: {input.content_length} >= {REMOTE_OBJECT_MAX_SIZE}"
            )
        if not is_allowed_content_type(input.content_type):
            raise ValidationError(f"invalid content type: {input.content_type}")
        remote_object: RemoteObject = project.remote_objects.filter(sha512=input.sha512).first()
        if remote_object is not None:
            if remote_object.status == models.RemoteObjectStatus.AVAILABLE:
                return remote_object
            else:
                remote_object.status = models.RemoteObjectStatus.UPLOADING
        else:
            remote_object = models.RemoteObject(
                project=project,
                sha512=input.sha512,
                content_length=input.content_length,
                content_type=input.content_type,
                name=input.name,
                status=models.RemoteObjectStatus.UPLOADING,
            )
        remote_object.save()
        remote_object.generate_presigned_post()
        return remote_object

    @safe_mutation
    def notify_uploaded_object(
        self, info: Info, input: NotifyUploadedObjectInput
    ) -> RemoteObject | OperationInfo:
        object = models.RemoteObject.objects.get(id=input.id.node_id)
        check_module_access(info, object.project_id, ModuleAccessLevel.Read)
        # check that object exists in s3
        object.mark_available_if_exists_in_s3()
        object.save()
        return object

    @safe_mutation
    def delete_object(self, info: Info, input: DeleteObjectInput) -> RemoteObject | OperationInfo:
        object = models.RemoteObject.objects.get(input.id)
        check_module_access(info, object.project_id, ModuleAccessLevel.Edit)
        object.delete()
        return object
