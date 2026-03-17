use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

/// Account auth mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccountAuthMode {
    /// Use ambient default credentials.
    #[default]
    Ambient,
    /// Use one named local profile.
    Profile,
    /// Use one OIDC identity flow.
    Oidc,
    /// Use one secret-backed credential reference.
    Secret,
}

impl From<AccountAuthModeJson> for AccountAuthMode {
    fn from(json: AccountAuthModeJson) -> Self {
        match json {
            AccountAuthModeJson::Ambient => Self::Ambient,
            AccountAuthModeJson::Profile => Self::Profile,
            AccountAuthModeJson::Oidc => Self::Oidc,
            AccountAuthModeJson::Secret => Self::Secret,
        }
    }
}

/// Account options.
#[derive(Debug, Clone, Default)]
pub struct AccountOptions {
    /// Provider identifier.
    pub provider: Option<String>,
    /// Authentication mode.
    pub mode: Option<AccountAuthMode>,
    /// Secret-backed credential reference.
    pub credential: Option<String>,
    /// Local profile name.
    pub profile: Option<String>,
    /// OIDC role or principal identifier.
    pub role_arn: Option<String>,
    /// Provider account or project id.
    pub account_id: Option<String>,
    /// Extra provider-specific account metadata.
    pub config: Option<Value>,
}

impl AccountOptions {
    /// Inherit unset account settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }
        if self.mode.is_none() {
            self.mode = parent.mode;
        }
        if self.credential.is_none() {
            self.credential = parent.credential.clone();
        }
        if self.profile.is_none() {
            self.profile = parent.profile.clone();
        }
        if self.role_arn.is_none() {
            self.role_arn = parent.role_arn.clone();
        }
        if self.account_id.is_none() {
            self.account_id = parent.account_id.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }
    }
}

impl From<&AccountJson> for AccountOptions {
    fn from(json: &AccountJson) -> Self {
        Self {
            provider: json.provider.clone(),
            mode: json.mode.map(AccountAuthMode::from),
            credential: json.credential.clone(),
            profile: json.profile.clone(),
            role_arn: json.role_arn.clone(),
            account_id: json.account_id.clone(),
            config: json.config.clone(),
        }
    }
}

/// Account auth mode JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum AccountAuthModeJson {
    /// Use ambient default credentials.
    Ambient,
    /// Use one named local profile.
    Profile,
    /// Use one OIDC identity flow.
    Oidc,
    /// Use one secret-backed credential reference.
    Secret,
}

/// A declared control-plane account.
///
/// Inputs: provider identity metadata and one auth mode declaration.
/// Outputs: a named account reference that stacks and domains can target.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AccountJson {
    /// Provider identifier.
    pub provider: Option<String>,
    /// Authentication mode.
    pub mode: Option<AccountAuthModeJson>,
    /// Secret-backed credential reference.
    pub credential: Option<String>,
    /// Local profile name.
    pub profile: Option<String>,
    /// OIDC role or principal identifier.
    pub role_arn: Option<String>,
    /// Provider account or project id.
    pub account_id: Option<String>,
    /// Extra provider-specific account metadata.
    pub config: Option<Value>,
}

/// Convert account declarations into normalized options.
pub fn account_options_from_json(
    json: &Option<IndexMap<String, AccountJson>>,
) -> IndexMap<String, AccountOptions> {
    json.as_ref()
        .map(|accounts| {
            accounts
                .iter()
                .map(|(name, account)| (name.clone(), AccountOptions::from(account)))
                .collect()
        })
        .unwrap_or_default()
}

/// Inherit one account map from a parent config.
pub fn extend_account_options(
    current: &mut IndexMap<String, AccountOptions>,
    parent: &IndexMap<String, AccountOptions>,
) {
    for (name, account) in parent {
        if let Some(existing) = current.get_mut(name) {
            existing.extend_from(account);
        } else {
            current.insert(name.clone(), account.clone());
        }
    }
}
