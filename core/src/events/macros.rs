//! Declarative macros for serde wrapper structs shared by [`gre`](crate::events::gre) and
//! [`client`](crate::events::client).

macro_rules! meta_payload_wrapper {
    ($meta:path, $wrapper:ident, $inner:ty, $field:ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $wrapper {
            #[serde(flatten)]
            pub meta: $meta,
            pub $field: $inner,
        }
    };
}

macro_rules! gre_payload_wrapper {
    ($wrapper:ident, $inner:ty, $field:ident) => {
        meta_payload_wrapper!(crate::events::gre::GreMeta, $wrapper, $inner, $field);
    };
}

macro_rules! client_payload_wrapper {
    ($wrapper:ident, $inner:ty, $field:ident) => {
        meta_payload_wrapper!(crate::events::client::ClientMeta, $wrapper, $inner, $field);
    };
}

macro_rules! gre_extra_wrapper {
    ($wrapper:ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $wrapper {
            #[serde(flatten)]
            pub meta: crate::events::gre::GreMeta,
            #[serde(flatten)]
            pub extra: std::collections::HashMap<String, serde_json::Value>,
        }
    };
}
