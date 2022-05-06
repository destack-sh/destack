import abc


# TODO @Cleanup: split or refine Controller definition, this is just a rough skeleton
class Controller(abc.ABC):
    """
    Base for controllers that manage some part of the artifact process for us.
    TODO is a single base "Controller" a good idea? mixins/subclasses?
    """

    def get_artifacts(self):
        raise NotImplementedError

    def get_artifact_versions(self):
        raise NotImplementedError

    def pull_artifact(self):
        raise NotImplementedError

    def push_artifact(self):
        raise NotImplementedError

    def deploy_model(self):
        raise NotImplementedError

    def get_deployed_model_handler_cls(self):
        raise NotImplementedError
