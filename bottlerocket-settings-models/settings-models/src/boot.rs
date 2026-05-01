//! Boot config settings structures.
use bottlerocket_model_derive::model;
use bottlerocket_modeled_types::{BootConfigKey, BootConfigValue};
use indexmap::IndexMap;

// Kernel boot settings
#[model(impl_default = true)]
pub struct BootSettingsV1 {
    reboot_to_reconcile: bool,
    #[serde(
        alias = "kernel",
        rename(serialize = "kernel"),
        default,
        skip_serializing_if = "Option::is_none"
    )]
    kernel_parameters: IndexMap<BootConfigKey, Vec<BootConfigValue>>,
    #[serde(
        alias = "init",
        rename(serialize = "init"),
        default,
        skip_serializing_if = "Option::is_none"
    )]
    init_parameters: IndexMap<BootConfigKey, Vec<BootConfigValue>>,
}
