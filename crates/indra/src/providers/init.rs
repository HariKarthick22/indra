use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::base::{Provider, ProviderMetadata, ProviderType};
#[cfg(feature = "local-inference")]
use super::local_inference::LocalInferenceProvider;
use super::ollama_def::OllamaProviderDef;
use super::openai_def::OpenAiProviderDef;
use super::provider_registry::ProviderRegistry;
use crate::config::ExtensionConfig;
use crate::{
    config::declarative_providers::register_declarative_providers,
    providers::provider_registry::ProviderEntry,
};
use anyhow::Result;
use tokio::sync::OnceCell;

static REGISTRY: OnceCell<RwLock<ProviderRegistry>> = OnceCell::const_new();

async fn init_registry() -> RwLock<ProviderRegistry> {
    let tls_config =
        crate::config::tls::provider_tls_config_from_config(crate::config::Config::global())
            .expect("failed to load provider TLS config");
    let mut registry = ProviderRegistry::new(tls_config).with_providers(|registry| {
        use super::inventory::registrations;

        // INDRA Sovereign Agent Workbench: Strictly 100% Local Model Providers
        #[cfg(feature = "local-inference")]
        registry.register::<LocalInferenceProvider>(false);
        registry.register_with_inventory::<OllamaProviderDef>(
            true,
            Some(registrations::ollama_inventory()),
        );
        registry.register_with_inventory::<OpenAiProviderDef>(
            true,
            Some(registrations::openai_inventory()),
        );
    });

    if let Err(e) = load_custom_providers_into_registry(&mut registry) {
        tracing::warn!("Failed to load custom providers: {}", e);
    }
    RwLock::new(registry)
}

fn load_custom_providers_into_registry(registry: &mut ProviderRegistry) -> Result<()> {
    register_declarative_providers(registry)
}

async fn get_registry() -> &'static RwLock<ProviderRegistry> {
    REGISTRY.get_or_init(init_registry).await
}

pub async fn providers() -> Vec<(ProviderMetadata, ProviderType)> {
    get_registry()
        .await
        .read()
        .unwrap()
        .all_metadata_with_types()
}

pub async fn refresh_custom_providers() -> Result<()> {
    let registry = get_registry().await;
    registry.write().unwrap().remove_custom_providers();

    if let Err(e) = load_custom_providers_into_registry(&mut registry.write().unwrap()) {
        tracing::warn!("Failed to refresh custom providers: {}", e);
        return Err(e);
    }

    tracing::info!("Custom providers refreshed");
    Ok(())
}

pub async fn get_from_registry(name: &str) -> Result<ProviderEntry> {
    let guard = get_registry().await.read().unwrap();
    guard
        .entries
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))
        .cloned()
}

pub async fn inventory_identity(name: &str) -> Result<super::inventory::InventoryIdentityInput> {
    get_from_registry(name).await?.inventory_identity()
}

pub async fn create(name: &str, extensions: Vec<ExtensionConfig>) -> Result<Arc<dyn Provider>> {
    let entry = get_from_registry(name).await?;
    entry.create(extensions).await
}

pub async fn create_with_working_dir(
    name: &str,
    extensions: Vec<ExtensionConfig>,
    working_dir: PathBuf,
) -> Result<Arc<dyn Provider>> {
    let entry = get_from_registry(name).await?;
    entry.create_with_working_dir(extensions, working_dir).await
}

pub async fn create_with_default_model(
    name: impl AsRef<str>,
    extensions: Vec<ExtensionConfig>,
) -> Result<Arc<dyn Provider>> {
    get_from_registry(name.as_ref())
        .await?
        .create_with_default_model(extensions)
        .await
}

pub async fn cleanup_provider(name: &str) -> Result<()> {
    let cleanup_fn = {
        let registry = get_registry().await.read().unwrap();
        registry
            .entries
            .get(name)
            .and_then(|entry| entry.cleanup.clone())
    };
    if let Some(cleanup) = cleanup_fn {
        return cleanup().await;
    }
    Ok(())
}

pub async fn create_with_named_model(
    provider_name: &str,
    extensions: Vec<ExtensionConfig>,
) -> Result<Arc<dyn Provider>> {
    create(provider_name, extensions).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::paths::Paths;
    use std::fs;

    #[tokio::test]
    async fn test_local_providers_registry_wiring() {
        let ollama = get_from_registry("ollama")
            .await
            .expect("ollama provider should be registered");
        let meta = ollama.metadata();
        assert_eq!(meta.name, "ollama");

        let openai = get_from_registry("openai")
            .await
            .expect("openai provider should be registered as local endpoint");
        let meta = openai.metadata();
        assert_eq!(meta.name, "openai");

        // Verify external cloud providers are NOT registered in INDRA sovereign workbench
        assert!(get_from_registry("anthropic").await.is_err());
        assert!(get_from_registry("databricks").await.is_err());
        assert!(get_from_registry("google").await.is_err());
        assert!(get_from_registry("bedrock").await.is_err());
        assert!(get_from_registry("huggingface").await.is_err());
    }

    #[tokio::test]
    async fn test_openai_compatible_providers_config_keys() {
        let providers_list = providers().await;
        let required_api_key_cases = vec![
            ("groq", "GROQ_API_KEY"),
            ("mistral", "MISTRAL_API_KEY"),
            ("custom_deepseek", "DEEPSEEK_API_KEY"),
        ];
        for (name, expected_key) in required_api_key_cases {
            if let Some((meta, _)) = providers_list.iter().find(|(m, _)| m.name == name) {
                assert!(
                    !meta.config_keys.is_empty(),
                    "{name} provider should have config keys"
                );
                assert_eq!(
                    meta.config_keys[0].name, expected_key,
                    "First config key for {name} should be {expected_key}, got {}",
                    meta.config_keys[0].name
                );
                assert!(
                    meta.config_keys[0].required,
                    "{expected_key} should be required"
                );
                assert!(
                    meta.config_keys[0].secret,
                    "{expected_key} should be secret"
                );
            } else {
                // Provider not registered; skip test for this provider
                continue;
            }
        }

        if let Some((meta, _)) = providers_list.iter().find(|(m, _)| m.name == "openai") {
            assert!(
                !meta.config_keys.is_empty(),
                "openai provider should have config keys"
            );
            assert_eq!(
                meta.config_keys[0].name, "OPENAI_API_KEY",
                "First config key for openai should be OPENAI_API_KEY"
            );
            assert!(
                !meta.config_keys[0].required,
                "OPENAI_API_KEY should be optional for local server support"
            );
            assert!(
                meta.config_keys[0].secret,
                "OPENAI_API_KEY should be secret"
            );
        }
    }

    #[tokio::test]
    async fn test_custom_provider_context_limit_is_applied_from_file() {
        let _guard = env_lock::lock_env([("GOOSE_PATH_ROOT", None::<&str>)]);
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        std::env::set_var("GOOSE_PATH_ROOT", temp_dir.path());

        let custom_dir = Paths::config_dir().join("custom_providers");
        fs::create_dir_all(&custom_dir).expect("custom providers dir should be created");

        let custom_inf = r#"{
  "name": "custom_inf",
  "engine": "openai",
  "display_name": "Custom Inf",
  "description": "test provider",
  "api_key_env": "",
  "base_url": "https://example.invalid/v1/chat/completions",
  "models": [
    {"name": "kimi-k2.5", "context_limit": 256000}
  ],
  "requires_auth": false
}"#;
        fs::write(custom_dir.join("custom_inf.json"), custom_inf)
            .expect("custom_inf.json should be written");

        let custom_zero = r#"{
  "name": "custom_zero",
  "engine": "openai",
  "display_name": "Custom Zero",
  "description": "test provider",
  "api_key_env": "",
  "base_url": "https://example.invalid/v1/chat/completions",
  "models": [
    {"name": "zero-model", "context_limit": 0}
  ],
  "requires_auth": false
}"#;
        fs::write(custom_dir.join("custom_zero.json"), custom_zero)
            .expect("custom_zero.json should be written");

        refresh_custom_providers()
            .await
            .expect("custom providers should refresh");

        let inf_entry = get_from_registry("custom_inf")
            .await
            .expect("custom_inf entry should exist");
        let provider = inf_entry
            .create(vec![])
            .await
            .expect("custom_inf provider should be created");
        assert_eq!(provider.get_context_limit("kimi-k2.5", None).await, 256_000);

        let zero_entry = get_from_registry("custom_zero")
            .await
            .expect("custom_zero entry should exist");
        let zero_provider = zero_entry
            .create(vec![])
            .await
            .expect("custom_zero provider should be created");
        assert_eq!(
            zero_provider.get_context_limit("zero-model", None).await,
            indra_providers::model::DEFAULT_CONTEXT_LIMIT
        );

        std::env::remove_var("GOOSE_PATH_ROOT");
    }

    #[tokio::test]
    async fn test_goose_context_limit_overrides_known_models_and_defaults() {
        let _guard = env_lock::lock_env([
            ("GOOSE_PATH_ROOT", None::<&str>),
            ("GOOSE_CONTEXT_LIMIT", Some("1000000")),
            ("GOOSE_MAX_TOKENS", None::<&str>),
            ("GOOSE_TEMPERATURE", None::<&str>),
            ("GOOSE_TOOLSHIM", None::<&str>),
            ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ("GOOSE_THINKING_EFFORT", None::<&str>),
        ]);

        let openai = get_from_registry("openai")
            .await
            .expect("openai provider should be registered");
        let openai_provider = openai
            .create(vec![])
            .await
            .expect("openai provider should be created");
        assert_eq!(
            openai_provider
                .get_context_limit("totally-unknown-model", Some(1_000_000))
                .await,
            1_000_000
        );

        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        std::env::set_var("GOOSE_PATH_ROOT", temp_dir.path());

        let custom_dir = Paths::config_dir().join("custom_providers");
        fs::create_dir_all(&custom_dir).expect("custom providers dir should be created");

        let custom_inf = r#"{
  "name": "custom_inf",
  "engine": "openai",
  "display_name": "Custom Inf",
  "description": "test provider",
  "api_key_env": "",
  "base_url": "https://example.invalid/v1/chat/completions",
  "models": [
    {"name": "kimi-k2.5", "context_limit": 256000}
  ],
  "requires_auth": false
}"#;
        fs::write(custom_dir.join("custom_inf.json"), custom_inf)
            .expect("custom_inf.json should be written");

        refresh_custom_providers()
            .await
            .expect("custom providers should refresh");

        let inf_entry = get_from_registry("custom_inf")
            .await
            .expect("custom_inf entry should exist");
        let inf_provider = inf_entry
            .create(vec![])
            .await
            .expect("custom_inf provider should be created");
        assert_eq!(
            inf_provider
                .get_context_limit("kimi-k2.5", Some(1_000_000))
                .await,
            1_000_000
        );

        std::env::remove_var("GOOSE_PATH_ROOT");
    }
}
