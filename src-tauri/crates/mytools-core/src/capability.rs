//! Capability registry — the single extension point of the foundation layer.
//!
//! A capability is `id -> { permission, handler }`. The registry owns
//! registration, policy evaluation and dispatch; it knows nothing about
//! any concrete plugin ecosystem.
//!
//! This module is deliberately free of Tauri types so it stays unit
//! testable in the pure-Rust core crate (see docs/02-architecture.md §4.1).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Host API version implemented by this build.
///
/// Plugins declare the version they need in their manifest; a plugin
/// asking for a newer major version is reported as incompatible instead
/// of silently degrading.
pub const HOST_API_VERSION: u32 = 1;

/// Risk level of a capability, ordered from least to most privileged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    /// Read host state or plugin-private data.
    Read,
    /// Mutate host state or the outside world.
    Write,
    /// Spawn processes or reach the network.
    Exec,
    /// Irreversible, destructive operations.
    Dangerous,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::Read => "read",
            Permission::Write => "write",
            Permission::Exec => "exec",
            Permission::Dangerous => "dangerous",
        }
    }
}

/// Where a caller came from. Drives the default permission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    /// Ships with the product.
    Official,
    /// Installed through the compatibility layer.
    Ecosystem,
    /// Installed by the user from an arbitrary source.
    Custom,
}

/// Policy outcome for a single capability request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Run the handler.
    Allow,
    /// Requires explicit user consent (not implemented yet).
    Prompt,
    /// Refuse the call.
    Deny,
}

/// Who is asking and from where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContext {
    /// Plugin id, when the call originates from a plugin.
    pub plugin_id: Option<String>,
    pub origin: Origin,
    /// Label of the webview window that issued the call.
    pub window_label: Option<String>,
}

impl CallContext {
    pub fn new(origin: Origin) -> Self {
        Self {
            plugin_id: None,
            origin,
            window_label: None,
        }
    }

    pub fn with_plugin(mut self, plugin_id: impl Into<String>) -> Self {
        self.plugin_id = Some(plugin_id.into());
        self
    }

    pub fn with_window(mut self, label: impl Into<String>) -> Self {
        self.window_label = Some(label.into());
        self
    }

    /// Storage namespace for plugin-private data.
    pub fn scope(&self) -> &str {
        self.plugin_id.as_deref().unwrap_or("app")
    }
}

/// Decides whether a capability may run.
pub trait Policy: Send + Sync {
    fn decide(&self, cap: &Capability, ctx: &CallContext) -> Decision;
}

/// Default policy: capability risk crossed with caller origin.
/// See docs/04-plugin-spec.md §5.3.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultPolicy;

impl Policy for DefaultPolicy {
    fn decide(&self, cap: &Capability, ctx: &CallContext) -> Decision {
        use Origin::*;
        use Permission::*;
        match (ctx.origin, cap.permission) {
            (Official, Read | Write | Exec) => Decision::Allow,
            (Official, Dangerous) => Decision::Prompt,
            (Ecosystem, Read | Write) => Decision::Allow,
            (Ecosystem, Exec) => Decision::Prompt,
            (Ecosystem, Dangerous) => Decision::Deny,
            (Custom, Read) => Decision::Allow,
            (Custom, Write | Exec) => Decision::Prompt,
            (Custom, Dangerous) if cap.custom_consent => Decision::Prompt,
            (Custom, Dangerous) => Decision::Deny,
        }
    }
}

/// Policy that honours the user's stored consent records.
///
/// A decision that would require confirmation becomes an outright allow
/// once the user granted that capability for that plugin.
pub struct GrantPolicy {
    base: DefaultPolicy,
    grants: std::sync::Arc<crate::permission::GrantStore>,
}

impl GrantPolicy {
    pub fn new(grants: std::sync::Arc<crate::permission::GrantStore>) -> Self {
        Self {
            base: DefaultPolicy,
            grants,
        }
    }
}

impl Policy for GrantPolicy {
    fn decide(&self, cap: &Capability, ctx: &CallContext) -> Decision {
        match self.base.decide(cap, ctx) {
            Decision::Prompt => {
                let granted = ctx
                    .plugin_id
                    .as_deref()
                    .map(|plugin| self.grants.has(plugin, &cap.id))
                    .unwrap_or(false);
                if granted {
                    Decision::Allow
                } else {
                    Decision::Prompt
                }
            }
            other => other,
        }
    }
}

/// Arguments and caller information handed to a capability handler.
#[derive(Debug, Clone)]
pub struct CapabilityCall {
    pub id: String,
    pub args: Value,
    pub ctx: CallContext,
}

impl CapabilityCall {
    pub fn arg_str(&self, key: &str) -> Option<&str> {
        self.args.get(key).and_then(Value::as_str)
    }
}

/// Implementation of a capability.
pub type Handler = Box<dyn Fn(&CapabilityCall) -> Result<Value, CapabilityError> + Send + Sync>;

/// A registered capability.
pub struct Capability {
    pub id: String,
    pub permission: Permission,
    pub description: &'static str,
    pub handler: Handler,
    custom_consent: bool,
}

impl Capability {
    pub fn new(
        id: &str,
        permission: Permission,
        description: &'static str,
        handler: Handler,
    ) -> Self {
        Self {
            id: id.to_string(),
            permission,
            description,
            handler,
            custom_consent: false,
        }
    }

    pub fn with_custom_consent(mut self) -> Self {
        self.custom_consent = true;
        self
    }
}
/// Failure modes of a capability call.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityError {
    #[error("unknown capability: {capability}")]
    Unknown { capability: String },
    #[error("capability {capability} denied: {reason}")]
    Denied { capability: String, reason: String },
    /// The call needs the user's consent; the host asks and retries.
    #[error("capability {capability} requires user authorization: {reason}")]
    ConsentRequired { capability: String, reason: String },
    #[error("invalid arguments: {message}")]
    InvalidArgs { message: String },
    #[error("capability {capability} failed: {message}")]
    HandlerFailed { capability: String, message: String },
    #[error("host api version {required} is newer than {available}")]
    VersionMismatch { required: u32, available: u32 },
}

impl CapabilityError {
    /// Stable error code shared with the frontend API surface.
    pub fn code(&self) -> &'static str {
        match self {
            CapabilityError::Unknown { .. } => "CAPABILITY_UNKNOWN",
            CapabilityError::Denied { .. } | CapabilityError::ConsentRequired { .. } => {
                "CAPABILITY_DENIED"
            }
            CapabilityError::InvalidArgs { .. } => "INVALID_ARGS",
            CapabilityError::HandlerFailed { .. } => "HANDLER_FAILED",
            CapabilityError::VersionMismatch { .. } => "API_VERSION",
        }
    }

    pub fn capability(&self) -> Option<&str> {
        match self {
            CapabilityError::Unknown { capability }
            | CapabilityError::Denied { capability, .. }
            | CapabilityError::ConsentRequired { capability, .. }
            | CapabilityError::HandlerFailed { capability, .. } => Some(capability),
            _ => None,
        }
    }

    /// True when the host should ask the user for consent.
    pub fn needs_consent(&self) -> bool {
        matches!(self, CapabilityError::ConsentRequired { .. })
    }

    pub fn invalid_args(message: impl Into<String>) -> Self {
        CapabilityError::InvalidArgs {
            message: message.into(),
        }
    }

    pub fn handler_failed(capability: &str, message: impl Into<String>) -> Self {
        CapabilityError::HandlerFailed {
            capability: capability.to_string(),
            message: message.into(),
        }
    }
}

/// Flat wire format: `{ code, message, capability?, consentRequired? }`.
impl Serialize for CapabilityError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("code", self.code())?;
        map.serialize_entry("message", &self.to_string())?;
        if let Some(capability) = self.capability() {
            map.serialize_entry("capability", capability)?;
        }
        if self.needs_consent() {
            map.serialize_entry("consentRequired", &true)?;
        }
        map.end()
    }
}

/// Capability ids are `<domain>.<action>`, lowercase, dot separated.
pub fn validate_capability_id(id: &str) -> bool {
    let mut parts = id.split('.');
    let (Some(domain), Some(action), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    domain_is_valid(domain) && action_is_valid(action)
}

fn domain_is_valid(part: &str) -> bool {
    !part.is_empty()
        && part.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && part
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

fn action_is_valid(part: &str) -> bool {
    !part.is_empty()
        && part.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && part
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Registry of capabilities plus the policy that guards them.
pub struct CapabilityRegistry {
    caps: HashMap<String, Capability>,
    policy: Box<dyn Policy>,
}

impl CapabilityRegistry {
    pub fn new(policy: Box<dyn Policy>) -> Self {
        Self {
            caps: HashMap::new(),
            policy,
        }
    }

    pub fn with_default_policy() -> Self {
        Self::new(Box::new(DefaultPolicy))
    }

    /// Registry that consults the user's stored consent records.
    pub fn with_grants(grants: std::sync::Arc<crate::permission::GrantStore>) -> Self {
        Self::new(Box::new(GrantPolicy::new(grants)))
    }

    /// Register a capability. Ids must be well formed and unique.
    pub fn register(&mut self, cap: Capability) -> Result<(), CapabilityError> {
        if !validate_capability_id(&cap.id) {
            return Err(CapabilityError::InvalidArgs {
                message: format!("invalid capability id: {}", cap.id),
            });
        }
        if self.caps.contains_key(&cap.id) {
            return Err(CapabilityError::InvalidArgs {
                message: format!("capability already registered: {}", cap.id),
            });
        }
        self.caps.insert(cap.id.clone(), cap);
        Ok(())
    }

    pub fn ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.caps.keys().map(String::as_str).collect();
        ids.sort_unstable();
        ids
    }

    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.caps.get(id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.caps.contains_key(id)
    }

    /// Evaluate policy and run the handler.
    ///
    /// A `Prompt` decision means the host has to ask the user; until the
    /// user consents the call is refused with `ConsentRequired`.
    pub fn call(
        &self,
        id: &str,
        args: &Value,
        ctx: &CallContext,
    ) -> Result<Value, CapabilityError> {
        let cap = self.caps.get(id).ok_or_else(|| CapabilityError::Unknown {
            capability: id.to_string(),
        })?;
        match self.policy.decide(cap, ctx) {
            Decision::Allow => (cap.handler)(&CapabilityCall {
                id: id.to_string(),
                args: args.clone(),
                ctx: ctx.clone(),
            }),
            Decision::Deny => Err(CapabilityError::Denied {
                capability: id.to_string(),
                reason: "not allowed for this origin".to_string(),
            }),
            Decision::Prompt => Err(CapabilityError::ConsentRequired {
                capability: id.to_string(),
                reason: "not authorized yet".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ok_handler() -> Handler {
        Box::new(|call: &CapabilityCall| Ok(json!({ "id": call.id, "args": call.args })))
    }

    fn failing_handler() -> Handler {
        Box::new(|_call: &CapabilityCall| Err(CapabilityError::handler_failed("x.y", "boom")))
    }

    #[test]
    fn validates_capability_ids() {
        for good in ["clipboard.write", "fs.read", "net.fetch", "a.b"] {
            assert!(validate_capability_id(good), "{good} should be valid");
        }
        for bad in [
            "",
            "clipboard",
            "clipboard.",
            ".write",
            "clipboard.write.extra",
            "Clipboard.write",
            "clipboard.Write",
            "clipboard.write!",
            "clip board.write",
        ] {
            assert!(!validate_capability_id(bad), "{bad} should be rejected");
        }
    }

    #[test]
    fn registers_and_lists_capabilities() {
        let mut reg = CapabilityRegistry::with_default_policy();
        reg.register(Capability::new(
            "clipboard.write",
            Permission::Write,
            "write clipboard",
            ok_handler(),
        ))
        .unwrap();
        reg.register(Capability::new(
            "system.path",
            Permission::Read,
            "well known path",
            ok_handler(),
        ))
        .unwrap();
        assert_eq!(reg.ids(), vec!["clipboard.write", "system.path"]);
        assert!(reg.contains("system.path"));
        assert!(!reg.contains("nope.nope"));
    }

    #[test]
    fn rejects_invalid_and_duplicate_ids() {
        let mut reg = CapabilityRegistry::with_default_policy();
        let err = reg
            .register(Capability::new(
                "Bad Id",
                Permission::Read,
                "bad",
                ok_handler(),
            ))
            .unwrap_err();
        assert_eq!(err.code(), "INVALID_ARGS");

        reg.register(Capability::new(
            "clipboard.write",
            Permission::Write,
            "first",
            ok_handler(),
        ))
        .unwrap();
        let err = reg
            .register(Capability::new(
                "clipboard.write",
                Permission::Write,
                "second",
                ok_handler(),
            ))
            .unwrap_err();
        assert_eq!(err.code(), "INVALID_ARGS");
    }

    #[test]
    fn unknown_capability_is_reported() {
        let reg = CapabilityRegistry::with_default_policy();
        let err = reg
            .call("net.fetch", &json!({}), &CallContext::new(Origin::Official))
            .unwrap_err();
        assert_eq!(err.code(), "CAPABILITY_UNKNOWN");
        assert_eq!(err.capability(), Some("net.fetch"));
    }

    #[test]
    fn allowed_capability_runs_handler_with_args() {
        let mut reg = CapabilityRegistry::with_default_policy();
        reg.register(Capability::new(
            "clipboard.write",
            Permission::Write,
            "write clipboard",
            ok_handler(),
        ))
        .unwrap();
        let out = reg
            .call(
                "clipboard.write",
                &json!({ "text": "hi" }),
                &CallContext::new(Origin::Ecosystem),
            )
            .unwrap();
        assert_eq!(out["id"], "clipboard.write");
        assert_eq!(out["args"]["text"], "hi");
    }

    #[test]
    fn handler_failure_is_wrapped() {
        let mut reg = CapabilityRegistry::with_default_policy();
        reg.register(Capability::new(
            "clipboard.write",
            Permission::Write,
            "write clipboard",
            failing_handler(),
        ))
        .unwrap();
        let err = reg
            .call(
                "clipboard.write",
                &json!({}),
                &CallContext::new(Origin::Ecosystem),
            )
            .unwrap_err();
        assert_eq!(err.code(), "HANDLER_FAILED");
    }

    #[test]
    fn default_policy_allows_read_write_for_ecosystem() {
        let policy = DefaultPolicy;
        for permission in [Permission::Read, Permission::Write] {
            let cap = Capability::new("x.y", permission, "d", ok_handler());
            assert_eq!(
                policy.decide(&cap, &CallContext::new(Origin::Ecosystem)),
                Decision::Allow
            );
        }
    }

    #[test]
    fn default_policy_denies_dangerous_for_ecosystem_and_custom() {
        let policy = DefaultPolicy;
        let cap = Capability::new("x.y", Permission::Dangerous, "d", ok_handler());
        assert_eq!(
            policy.decide(&cap, &CallContext::new(Origin::Ecosystem)),
            Decision::Deny
        );
        assert_eq!(
            policy.decide(&cap, &CallContext::new(Origin::Custom)),
            Decision::Deny
        );
    }

    #[test]
    fn default_policy_prompts_for_privileged_combinations() {
        let policy = DefaultPolicy;
        let exec = Capability::new("x.y", Permission::Exec, "d", ok_handler());
        assert_eq!(
            policy.decide(&exec, &CallContext::new(Origin::Ecosystem)),
            Decision::Prompt
        );
        let write = Capability::new("x.y", Permission::Write, "d", ok_handler());
        assert_eq!(
            policy.decide(&write, &CallContext::new(Origin::Custom)),
            Decision::Prompt
        );
        let dangerous = Capability::new("x.y", Permission::Dangerous, "d", ok_handler());
        assert_eq!(
            policy.decide(&dangerous, &CallContext::new(Origin::Official)),
            Decision::Prompt
        );
    }

    #[test]
    fn host_opted_in_dangerous_capability_requires_custom_consent() {
        let dir = tempfile::TempDir::new().unwrap();
        let grants = std::sync::Arc::new(crate::permission::GrantStore::load(
            dir.path().join("permissions.json"),
        ));
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let handler_calls = calls.clone();
        let mut reg = CapabilityRegistry::with_grants(grants.clone());
        reg.register(
            Capability::new(
                "fs.renamemany",
                Permission::Dangerous,
                "rename",
                Box::new(move |_| {
                    handler_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(json!(true))
                }),
            )
            .with_custom_consent(),
        )
        .unwrap();
        let ctx = CallContext::new(Origin::Custom).with_plugin("demo");
        assert!(reg
            .call("fs.renamemany", &json!({}), &ctx)
            .unwrap_err()
            .needs_consent());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        grants.grant("demo", "fs.renamemany").unwrap();
        assert_eq!(
            reg.call("fs.renamemany", &json!({}), &ctx).unwrap(),
            json!(true)
        );
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        let ecosystem = CallContext::new(Origin::Ecosystem).with_plugin("demo");
        assert!(!reg
            .call("fs.renamemany", &json!({}), &ecosystem)
            .unwrap_err()
            .needs_consent());
        let unflagged = Capability::new(
            "fs.renamemany",
            Permission::Dangerous,
            "rename",
            ok_handler(),
        );
        assert_eq!(
            GrantPolicy::new(grants).decide(&unflagged, &ctx),
            Decision::Deny
        );
    }

    #[test]
    fn prompt_is_reported_as_consent_required() {
        let mut reg = CapabilityRegistry::with_default_policy();
        reg.register(Capability::new(
            "fs.read",
            Permission::Exec,
            "read file",
            ok_handler(),
        ))
        .unwrap();
        let err = reg
            .call("fs.read", &json!({}), &CallContext::new(Origin::Ecosystem))
            .unwrap_err();
        assert_eq!(err.code(), "CAPABILITY_DENIED");
        assert!(err.needs_consent());
    }

    #[test]
    fn a_stored_grant_turns_prompt_into_allow() {
        let dir = tempfile::TempDir::new().unwrap();
        let grants = std::sync::Arc::new(crate::permission::GrantStore::load(
            dir.path().join("permissions.json"),
        ));
        grants.grant("demo", "fs.read").unwrap();
        let mut reg = CapabilityRegistry::with_grants(grants);
        reg.register(Capability::new(
            "fs.read",
            Permission::Exec,
            "read file",
            ok_handler(),
        ))
        .unwrap();
        let ctx = CallContext::new(Origin::Ecosystem).with_plugin("demo");
        let out = reg.call("fs.read", &json!({}), &ctx).unwrap();
        assert_eq!(out["id"], "fs.read");
    }

    #[test]
    fn without_a_grant_the_call_needs_consent() {
        let dir = tempfile::TempDir::new().unwrap();
        let grants = std::sync::Arc::new(crate::permission::GrantStore::load(
            dir.path().join("permissions.json"),
        ));
        let mut reg = CapabilityRegistry::with_grants(grants);
        reg.register(Capability::new(
            "fs.read",
            Permission::Exec,
            "read file",
            ok_handler(),
        ))
        .unwrap();
        let ctx = CallContext::new(Origin::Ecosystem).with_plugin("demo");
        let err = reg.call("fs.read", &json!({}), &ctx).unwrap_err();
        assert!(err.needs_consent());
    }

    #[test]
    fn scope_falls_back_to_app() {
        let ctx = CallContext::new(Origin::Official);
        assert_eq!(ctx.scope(), "app");
        let ctx = ctx.with_plugin("demo");
        assert_eq!(ctx.scope(), "demo");
    }
}
