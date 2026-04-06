//! Modeled types for creating Kubernetes settings.
use bottlerocket_model_derive::model;
use bottlerocket_modeled_types::{
    CpuManagerPolicy, CredentialProvider, DNSDomain, Identifier, IntegerPercent, KernelCpuSetValue,
    KubernetesAuthenticationMode, KubernetesBootstrapToken, KubernetesCPUManagerPolicyOption,
    KubernetesCloudProvider, KubernetesClusterDnsIp, KubernetesClusterName,
    KubernetesDurationValue, KubernetesEvictionKey, KubernetesHostnameOverrideSource,
    KubernetesIdsPerPodValue, KubernetesLabelKey, KubernetesLabelValue,
    KubernetesMemoryManagerPolicy, KubernetesMemoryReservation, KubernetesMemorySwapBehavior,
    KubernetesQuantityValue, KubernetesReservedResourceKey, KubernetesTaintValue,
    KubernetesThresholdValue, KubernetesTopologyManagerPolicyOptions, NonNegativeInteger,
    SingleLineString, TopologyManagerPolicy, TopologyManagerScope, Url, ValidBase64,
    ValidLinuxHostname,
};
use bottlerocket_settings_sdk::{GenerateResult, SettingsModel};
use bottlerocket_string_impls_for::string_impls_for;

use base64::Engine;
use self::de::deserialize_node_taints;
use std::collections::HashMap;
use std::fmt;
use std::net::IpAddr;

mod de;

/// Blocked KubeletConfiguration top-level keys and the reason they are blocked.
/// Each entry is `(kubelet_field_name, reason)` where reason is either the corresponding
/// Bottlerocket typed setting or a note that the field is Bottlerocket-managed.
const BLOCKED_KEYS: &[(&str, &str)] = &[
    ("clusterDomain", "settings.kubernetes.cluster-domain"),
    ("clusterDNS", "settings.kubernetes.cluster-dns-ip"),
    ("maxPods", "settings.kubernetes.max-pods"),
    ("staticPodPath", "settings.kubernetes.static-pods"),
    ("podPidsLimit", "settings.kubernetes.pod-pids-limit"),
    ("kubeReserved", "settings.kubernetes.kube-reserved"),
    ("systemReserved", "settings.kubernetes.system-reserved"),
    ("evictionHard", "settings.kubernetes.eviction-hard"),
    ("evictionSoft", "settings.kubernetes.eviction-soft"),
    ("evictionSoftGracePeriod", "settings.kubernetes.eviction-soft-grace-period"),
    ("evictionMaxPodGracePeriod", "settings.kubernetes.eviction-max-pod-grace-period"),
    ("imageGCHighThresholdPercent", "settings.kubernetes.image-gc-high-threshold-percent"),
    ("imageGCLowThresholdPercent", "settings.kubernetes.image-gc-low-threshold-percent"),
    ("cpuManagerPolicy", "settings.kubernetes.cpu-manager-policy"),
    ("cpuManagerReconcilePeriod", "settings.kubernetes.cpu-manager-reconcile-period"),
    ("topologyManagerPolicy", "settings.kubernetes.topology-manager-policy"),
    ("topologyManagerScope", "settings.kubernetes.topology-manager-scope"),
    ("cpuCFSQuota", "settings.kubernetes.cpu-cfs-quota-enforced"),
    ("allowedUnsafeSysctls", "settings.kubernetes.allowed-unsafe-sysctls"),
    ("authentication", "Bottlerocket-managed, not user-configurable"),
    ("authorization", "Bottlerocket-managed, not user-configurable"),
    ("tlsCertFile", "Bottlerocket-managed"),
    ("tlsPrivateKeyFile", "Bottlerocket-managed"),
    ("containerRuntimeEndpoint", "Bottlerocket-managed"),
    ("imageServiceEndpoint", "Bottlerocket-managed"),
    ("podInfraContainerImage", "Bottlerocket-managed"),
    ("resolvConf", "Bottlerocket-managed"),
    ("registerNode", "Bottlerocket-managed"),
];

/// A validated kubelet extra config value. Stores the original base64 string.
/// Validates at construction time that the value is valid base64, decodes to valid YAML,
/// is a YAML mapping, and contains no blocked KubeletConfiguration keys.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ValidKubeletExtraConfig {
    inner: String,
}

impl TryFrom<&str> for ValidKubeletExtraConfig {
    type Error = String;

    fn try_from(input: &str) -> std::result::Result<Self, Self::Error> {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(input)
            .map_err(|e| format!("Failed to decode kubelet-extra-config: {e}"))?;
        let yaml_value: serde_yaml::Value = serde_yaml::from_slice(&decoded)
            .map_err(|e| format!("Failed to parse kubelet-extra-config YAML: {e}"))?;
        let mapping = yaml_value
            .as_mapping()
            .ok_or("kubelet-extra-config must be a YAML mapping, not an array or scalar")?;
        for (key, reason) in BLOCKED_KEYS {
            if mapping.contains_key(&serde_yaml::Value::String((*key).to_string())) {
                return Err(format!(
                    "kubelet-extra-config contains blocked key '{key}': {reason}"
                ));
            }
        }
        Ok(Self {
            inner: input.to_string(),
        })
    }
}

string_impls_for!(ValidKubeletExtraConfig, "ValidKubeletExtraConfig");

// Kubernetes static pod manifest settings
#[model]
pub struct StaticPod {
    enabled: bool,
    manifest: ValidBase64,
}

#[model(impl_default = true)]
pub struct KubernetesSettingsV1 {
    // Settings that must be specified via user data or through API requests.  Not all settings are
    // useful for all modes. For example, in standalone mode the user does not need to specify any
    // cluster information, and the bootstrap token is only needed for TLS authentication mode.
    cluster_name: KubernetesClusterName,
    cluster_certificate: ValidBase64,
    api_server: Url,
    node_labels: HashMap<KubernetesLabelKey, KubernetesLabelValue>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_node_taints"
    )]
    node_taints: HashMap<KubernetesLabelKey, Vec<KubernetesTaintValue>>,
    static_pods: HashMap<Identifier, StaticPod>,
    authentication_mode: KubernetesAuthenticationMode,
    bootstrap_token: KubernetesBootstrapToken,
    standalone_mode: bool,
    eviction_hard: HashMap<KubernetesEvictionKey, KubernetesThresholdValue>,
    eviction_soft: HashMap<KubernetesEvictionKey, KubernetesThresholdValue>,
    eviction_soft_grace_period: HashMap<KubernetesEvictionKey, KubernetesDurationValue>,
    eviction_max_pod_grace_period: NonNegativeInteger,
    kube_reserved: HashMap<KubernetesReservedResourceKey, KubernetesQuantityValue>,
    system_reserved: HashMap<KubernetesReservedResourceKey, KubernetesQuantityValue>,
    allowed_unsafe_sysctls: Vec<SingleLineString>,
    server_tls_bootstrap: bool,
    cloud_provider: KubernetesCloudProvider,
    registry_qps: i32,
    registry_burst: i32,
    event_qps: i32,
    event_burst: i32,
    kube_api_qps: i32,
    kube_api_burst: i32,
    container_log_max_size: KubernetesQuantityValue,
    container_log_max_files: i32,
    container_log_max_workers: i32,
    container_log_monitor_interval: KubernetesDurationValue,
    cpu_cfs_quota_enforced: bool,
    cpu_manager_policy: CpuManagerPolicy,
    cpu_manager_reconcile_period: KubernetesDurationValue,
    cpu_manager_policy_options: Vec<KubernetesCPUManagerPolicyOption>,
    topology_manager_scope: TopologyManagerScope,
    topology_manager_policy: TopologyManagerPolicy,
    topology_manager_policy_options: KubernetesTopologyManagerPolicyOptions,
    pod_pids_limit: i64,
    image_gc_high_threshold_percent: IntegerPercent,
    image_gc_low_threshold_percent: IntegerPercent,
    image_minimum_gc_age: KubernetesDurationValue,
    image_maximum_gc_age: KubernetesDurationValue,
    provider_id: Url,
    log_level: u8,
    credential_providers: HashMap<Identifier, CredentialProvider>,
    server_certificate: ValidBase64,
    server_key: ValidBase64,
    shutdown_grace_period: KubernetesDurationValue,
    shutdown_grace_period_for_critical_pods: KubernetesDurationValue,
    memory_manager_reserved_memory: HashMap<Identifier, KubernetesMemoryReservation>,
    memory_manager_policy: KubernetesMemoryManagerPolicy,
    reserved_cpus: KernelCpuSetValue,
    memory_swap_behavior: KubernetesMemorySwapBehavior,
    hostname_override_source: KubernetesHostnameOverrideSource,
    seccomp_default: bool,
    device_ownership_from_security_context: bool,
    single_process_oom_kill: bool,
    static_pods_enabled: bool,
    max_pods: u32,
    cluster_dns_ip: KubernetesClusterDnsIp,
    cluster_domain: DNSDomain,
    node_ip: IpAddr,
    pod_infra_container_image: SingleLineString,
    hostname_override: ValidLinuxHostname,
    ids_per_pod: KubernetesIdsPerPodValue,
    max_parallel_image_pulls: i32,
    kubelet_extra_config: ValidKubeletExtraConfig,
}

/// Errors that can occur when validating Kubernetes settings.
#[derive(Debug)]
pub enum KubernetesSettingsError {
    /// The kubelet-extra-config value could not be decoded from base64.
    Base64Decode(String),
    /// The decoded kubelet-extra-config is not valid YAML.
    YamlParse(String),
    /// The decoded kubelet-extra-config is not a YAML mapping.
    NotAMapping,
    /// The kubelet-extra-config contains a key that is managed by a typed Bottlerocket setting.
    BlockedKey { key: String, reason: String },
}

impl fmt::Display for KubernetesSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base64Decode(e) => write!(f, "Failed to decode kubelet-extra-config: {}", e),
            Self::YamlParse(e) => write!(f, "Failed to parse kubelet-extra-config YAML: {}", e),
            Self::NotAMapping => write!(
                f,
                "kubelet-extra-config must be a YAML mapping, not an array or scalar"
            ),
            Self::BlockedKey { key, reason } => write!(
                f,
                "kubelet-extra-config contains blocked key '{}': {}",
                key, reason
            ),
        }
    }
}

impl std::error::Error for KubernetesSettingsError {}

type Result<T> = std::result::Result<T, KubernetesSettingsError>;

impl SettingsModel for KubernetesSettingsV1 {
    type PartialKind = Self;
    type ErrorKind = KubernetesSettingsError;

    fn get_version() -> &'static str {
        "v1"
    }

    fn set(_current_value: Option<Self>, _target: Self) -> Result<()> {
        // allow anything that parses as KubernetesSettingsV1
        Ok(())
    }

    fn generate(
        existing_partial: Option<Self::PartialKind>,
        _dependent_settings: Option<serde_json::Value>,
    ) -> Result<GenerateResult<Self::PartialKind, Self>> {
        // TODO this should eventually replace `pluto`
        Ok(GenerateResult::Complete(
            existing_partial.unwrap_or_default(),
        ))
    }

    fn validate(_value: Self, _validated_settings: Option<serde_json::Value>) -> Result<()> {
        // Validation is performed at deserialization time by ValidKubeletExtraConfig.
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use base64::Engine;
    use bottlerocket_modeled_types::KubernetesHostnameOverrideSource;

    #[test]
    fn test_generate_kubernetes() {
        let generated = KubernetesSettingsV1::generate(None, None).unwrap();

        assert_eq!(
            generated,
            GenerateResult::Complete(KubernetesSettingsV1 {
                cluster_name: None,
                cluster_certificate: None,
                api_server: None,
                node_labels: None,
                node_taints: None,
                static_pods: None,
                authentication_mode: None,
                bootstrap_token: None,
                standalone_mode: None,
                eviction_hard: None,
                eviction_soft: None,
                eviction_soft_grace_period: None,
                eviction_max_pod_grace_period: None,
                kube_reserved: None,
                system_reserved: None,
                allowed_unsafe_sysctls: None,
                server_tls_bootstrap: None,
                cloud_provider: None,
                registry_qps: None,
                registry_burst: None,
                event_qps: None,
                event_burst: None,
                kube_api_qps: None,
                kube_api_burst: None,
                container_log_max_size: None,
                container_log_max_files: None,
                container_log_max_workers: None,
                container_log_monitor_interval: None,
                cpu_cfs_quota_enforced: None,
                cpu_manager_policy: None,
                cpu_manager_reconcile_period: None,
                cpu_manager_policy_options: None,
                topology_manager_scope: None,
                topology_manager_policy: None,
                topology_manager_policy_options: None,
                pod_pids_limit: None,
                image_gc_high_threshold_percent: None,
                image_gc_low_threshold_percent: None,
                image_maximum_gc_age: None,
                image_minimum_gc_age: None,
                provider_id: None,
                log_level: None,
                credential_providers: None,
                server_certificate: None,
                server_key: None,
                shutdown_grace_period: None,
                shutdown_grace_period_for_critical_pods: None,
                memory_manager_reserved_memory: None,
                memory_manager_policy: None,
                reserved_cpus: None,
                memory_swap_behavior: None,
                max_pods: None,
                cluster_dns_ip: None,
                cluster_domain: None,
                node_ip: None,
                pod_infra_container_image: None,
                hostname_override: None,
                hostname_override_source: None,
                seccomp_default: None,
                device_ownership_from_security_context: None,
                single_process_oom_kill: None,
                static_pods_enabled: None,
                ids_per_pod: None,
                max_parallel_image_pulls: None,
                kubelet_extra_config: None,
            })
        );
    }

    #[test]
    fn test_serde_kubernetes() {
        let test_json = r#"{
            "cluster-name": "my-cluster",
            "api-server": "https://example.com",
            "hostname-override-source": "private-dns-name"
        }"#;

        let kubernetes: KubernetesSettingsV1 = serde_json::from_str(test_json).unwrap();

        assert_eq!(
            kubernetes,
            KubernetesSettingsV1 {
                cluster_name: Some("my-cluster".try_into().unwrap()),
                api_server: Some("https://example.com".try_into().unwrap()),
                hostname_override_source: Some(KubernetesHostnameOverrideSource::PrivateDNSName),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_valid_extra_config_passes_validation() {
        let yaml = "featureGates:\n  SomeGate: true\n";
        let encoded = base64::engine::general_purpose::STANDARD.encode(yaml);
        assert!(ValidKubeletExtraConfig::try_from(encoded.as_str()).is_ok());
    }

    #[test]
    fn test_invalid_base64_rejected_at_type_boundary() {
        assert!(ValidKubeletExtraConfig::try_from("not valid base64!!!").is_err());
    }

    #[test]
    fn test_invalid_yaml_after_base64_decode_rejected() {
        let bad_yaml = ":\n  :\n- {{\ninvalid:: yaml::: [";
        let encoded = base64::engine::general_purpose::STANDARD.encode(bad_yaml);
        let err = ValidKubeletExtraConfig::try_from(encoded.as_str()).unwrap_err();
        assert!(err.contains("parse"), "Expected YAML parse error, got: {err}");
    }

    #[test]
    fn test_binary_garbage_rejected_as_yaml_parse_error() {
        let garbage: [u8; 16] = [0xff, 0xfe, 0x00, 0x01, 0x80, 0x90, 0xa0, 0xb0,
                                  0xc0, 0xd0, 0xe0, 0xf0, 0x7f, 0x1b, 0x03, 0x04];
        let encoded = base64::engine::general_purpose::STANDARD.encode(garbage);
        let err = ValidKubeletExtraConfig::try_from(encoded.as_str()).unwrap_err();
        assert!(err.contains("parse") || err.contains("YAML"), "Expected YAML parse error, got: {err}");
    }

    #[test]
    fn test_yaml_array_rejected_as_not_a_mapping() {
        let yaml = "- item1\n- item2\n";
        let encoded = base64::engine::general_purpose::STANDARD.encode(yaml);
        let err = ValidKubeletExtraConfig::try_from(encoded.as_str()).unwrap_err();
        assert!(err.contains("mapping"), "Expected not-a-mapping error, got: {err}");
    }

    #[test]
    fn test_yaml_scalar_rejected_as_not_a_mapping() {
        let yaml = "just a string";
        let encoded = base64::engine::general_purpose::STANDARD.encode(yaml);
        let err = ValidKubeletExtraConfig::try_from(encoded.as_str()).unwrap_err();
        assert!(err.contains("mapping"), "Expected not-a-mapping error, got: {err}");
    }

    #[test]
    fn test_blocked_key_cluster_domain_rejected() {
        let yaml = "clusterDomain: example.com\n";
        let encoded = base64::engine::general_purpose::STANDARD.encode(yaml);
        let err = ValidKubeletExtraConfig::try_from(encoded.as_str()).unwrap_err();
        assert!(err.contains("clusterDomain"), "Error should name the blocked key: {err}");
        assert!(
            err.contains("settings.kubernetes.cluster-domain"),
            "Error should name the typed setting: {err}"
        );
    }

    #[test]
    fn test_blocked_key_authentication_rejected() {
        let yaml = "authentication:\n  mode: AlwaysAllow\n";
        let encoded = base64::engine::general_purpose::STANDARD.encode(yaml);
        let err = ValidKubeletExtraConfig::try_from(encoded.as_str()).unwrap_err();
        assert!(err.contains("authentication"), "Error should name the blocked key: {err}");
        assert!(
            err.contains("Bottlerocket-managed"),
            "Error should state the field is Bottlerocket-managed: {err}"
        );
    }

    #[test]
    fn test_blocked_key_rejected_at_deserialization() {
        // Verify that serde deserialization of a JSON payload with a blocked key fails
        let yaml = "clusterDomain: example.com\n";
        let encoded = base64::engine::general_purpose::STANDARD.encode(yaml);
        let json = format!(r#"{{"kubelet-extra-config": "{}"}}"#, encoded);
        let result: std::result::Result<KubernetesSettingsV1, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "Deserialization should fail for blocked key");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("clusterDomain"), "Error should name the blocked key: {err}");
    }
}
