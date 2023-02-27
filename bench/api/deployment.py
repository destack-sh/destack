from strawberry import auto
from strawberry_django_plus import gql
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.organization import Organization
from bench.api.project import Project, ProjectVersion
from bench.api.statement import Statement
from bench.api.user import User

DeploymentStatus = gql.enum(models.DeploymentStatus)
DeploymentType = gql.enum(models.DeploymentType)


@gql.django.type(models.Deployment)
class Deployment(gql.Node):
    owner: Organization | User
    project: Project
    project_version: ProjectVersion
    created_at: auto
    updated_at: auto
    status: DeploymentStatus
    type: DeploymentType
    deploy_all_statements: bool
    deployed_statements: list[Statement]


@gql.input
class DeploymentSetDeployAllStatementsInput(gql.NodeInput):
    deploy_all_statements: bool


@gql.input
class DeploymentAddStatementInput(gql.NodeInput):
    statement_id: gql.ID


@gql.input
class DeploymentRemoveStatementInput(gql.NodeInput):
    statement_id: gql.ID


@gql.input
class DeployInput(gql.NodeInput):
    status: DeploymentStatus


@gql.type
class DeploymentMutation:
    @gql.mutation
    def set_deploy_all_statements(
        self, info, input: DeploymentSetDeployAllStatementsInput
    ) -> Deployment | OperationInfo:
        deployment = models.Deployment.objects.get(id=input.id)
        deployment.deploy_all_statements = input.deploy_all_statements
        deployment.save()
        return deployment

    @gql.mutation
    def add_deployed_statement(
        self, info, input: DeploymentAddStatementInput
    ) -> Deployment | OperationInfo:
        models.DeployedStatement.objects.create(
            deployment_id=input.id,
            statement_id=input.statement_id,
            on_conflict_do_nothing=True,
        )
        return models.Deployment.objects.get(id=input.id)

    @gql.mutation
    def remove_deployed_statement(
        self, info, input: DeploymentRemoveStatementInput
    ) -> Deployment | OperationInfo:
        models.DeployedStatement.objects.filter(
            deployment_id=input.id,
            statement_id=input.statement_id,
        ).delete()
        return models.Deployment.objects.get(id=input.id)

    @gql.mutation
    def update_deployment(self, info, input: DeployInput) -> Deployment | OperationInfo:
        deployment = models.Deployment.objects.get(id=input.id)
        deployment.type = DeploymentType.MANUAL
        deployment.status = input.status
        deployment.save()
        return deployment
