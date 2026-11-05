use crate::host::{Event, Input};
use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

/// A modal question the host asks the person, with an answer allowing and one refusing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dialog {
    /// The dialog's identity, which the answer includes.
    pub id: String,
    /// The question.
    pub title: String,
    /// The text below the question.
    pub body: String,
    /// The label of the answer allowing.
    pub allow: String,
    /// The label of the answer refusing.
    pub deny: String,
}

impl Dialog {
    /// Ask the question and send the person's answer to the host.
    pub fn ask(self, app: &AppHandle, input: Input) {
        // show the question with the two answers, the allowing one as the default
        let answering = app.clone();
        let id = self.id;
        app.dialog()
            .message(self.body)
            .title(self.title)
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(self.allow, self.deny))
            .show(move |is_allowed| {
                // send the answer
                Event::DialogAnswered { id, is_allowed }.report(&answering, &input);
            });
    }
}
