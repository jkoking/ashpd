use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    backend::request::{Request, RequestImpl},
    desktop::{file_chooser::Choice, request::Response, Icon},
    zvariant::{self, DeserializeDict, OwnedObjectPath, SerializeDict},
    AppID, WindowIdentifierType,
};

#[derive(DeserializeDict, zvariant::Type)]
#[zvariant(signature = "dict")]
pub struct AccessOptions {
    modal: Option<bool>,
    deny_label: Option<String>,
    grant_label: Option<String>,
    icon: Option<String>,
    choices: Option<Vec<Choice>>,
}

impl AccessOptions {
    pub fn is_modal(&self) -> Option<bool> {
        self.modal
    }

    pub fn deny_label(&self) -> Option<&str> {
        self.deny_label.as_deref()
    }

    pub fn grant_label(&self) -> Option<&str> {
        self.grant_label.as_deref()
    }

    pub fn icon(&self) -> Option<Icon> {
        self.icon.as_ref().map(|i| Icon::with_names(&[i]))
    }

    pub fn choices(&self) -> &[Choice] {
        self.choices.as_deref().unwrap_or_default()
    }
}

#[derive(SerializeDict, Debug, zvariant::Type, Default)]
#[zvariant(signature = "dict")]
pub struct AccessResponse {
    choices: Option<Vec<(String, String)>>,
}

impl AccessResponse {
    /// Adds a selected choice (key, value).
    #[must_use]
    pub fn choice(mut self, key: &str, value: &str) -> Self {
        self.choices
            .get_or_insert_with(Vec::new)
            .push((key.to_owned(), value.to_owned()));
        self
    }
}

#[async_trait]
pub trait AccessImpl: RequestImpl {
    async fn access_dialog(
        &self,
        app_id: Option<AppID>,
        window_identifier: Option<WindowIdentifierType>,
        title: String,
        subtitle: String,
        body: String,
        options: AccessOptions,
    ) -> Response<AccessResponse>;
}

pub struct AccessInterface {
    imp: Arc<dyn AccessImpl>,
    cnx: zbus::Connection,
}

impl AccessInterface {
    pub fn new(imp: impl AccessImpl + 'static, cnx: zbus::Connection) -> Self {
        Self {
            imp: Arc::new(imp),
            cnx,
        }
    }
}

#[zbus::interface(name = "org.freedesktop.impl.portal.Access")]
impl AccessInterface {
    #[dbus_interface(property(emits_changed_signal = "const"), name = "version")]
    fn version(&self) -> u32 {
        1 // TODO: Is this correct?
    }

    async fn access_dialog(
        &self,
        handle: OwnedObjectPath,
        app_id: &str,
        window_identifier: &str,
        title: String,
        subtitle: String,
        body: String,
        options: AccessOptions,
    ) -> Response<AccessResponse> {
        let window_identifier = WindowIdentifierType::from_maybe_str(window_identifier);
        let app_id = AppID::from_maybe_str(app_id);
        let imp = Arc::clone(&self.imp);

        Request::spawn(
            "Access::AccessDialog",
            &self.cnx,
            handle,
            Arc::clone(&self.imp),
            async move {
                imp.access_dialog(app_id, window_identifier, title, subtitle, body, options)
                    .await
            },
        )
        .await
        .unwrap_or(Response::other())
    }
}
