use zed::LanguageServerId;
use zed_extension_api as zed;

struct DestackZedExtension;

impl zed::Extension for DestackZedExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        // prefer the workspace-installed CLI server via PATH
        Ok(zed::Command {
            command: "destack_extension_lsp".into(),
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(DestackZedExtension);
