//! Metadata in the 1C:EDT project format.
//!
//! One configuration, two spellings: the configurator dumps `MetaDataObject`
//! with a `Properties` block and prefixed type tokens, EDT writes one
//! `mdclass:<Kind>` root with camelCase properties as direct children and
//! bare tokens. The readers here produce the same model the configurator
//! readers do, so nothing above this module knows which spelling it saw.
//!
//! Written against the files an EDT project actually holds on disk and the 1C
//! documentation; no third-party reader of the format was consulted.

pub(crate) mod helpers;
pub(crate) mod metadata_object;
pub(crate) mod properties;
pub(crate) mod types;
