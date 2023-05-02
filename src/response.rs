use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse<T> {
    pub total_size: i32,
    pub done: bool,
    pub next_records_url: Option<String>,
    pub records: Vec<T>,
}

#[derive(Deserialize, Debug)]
pub struct UpsertResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Deserialize, Debug)]
pub struct CompositeResponse {
    pub id: Option<String>,
    pub success: bool,
    pub errors: Vec<RecordErrorResponse>,
}

#[derive(Serialize, Debug)]
pub struct RecordRequestAttribute {
    #[serde(rename = "type")]
    pub sobject_type: String,
}
#[derive(Serialize, Debug)]
pub struct RecordRequest<T> {
    pub attributes: RecordRequestAttribute,
    #[serde(flatten)]
    pub record: T,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CompositeBodyRequest<T> {
    pub all_or_none: bool,
    pub records: Vec<T>,
}

#[derive(Deserialize, Debug)]
pub struct RecordsResponse {
    pub id: String,
    pub success: bool,
    pub created: Option<bool>,
    pub errors: RecordErrorResponse,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RecordErrorResponse {
    pub message: String,
    pub status_code: String,
    pub fields: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct TokenErrorResponse {
    pub error: String,
    pub error_description: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub message: Value,
    pub error_code: String,
    pub fields: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub id: String,
    pub issued_at: String,
    pub access_token: String,
    pub instance_url: String,
    pub signature: String,
    pub token_type: Option<String>,
}

#[derive(Debug)]
pub struct AccessToken {
    pub token_type: String,
    pub value: String,
    pub issued_at: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DescribeResponse {
    pub activateable: bool,
    pub child_relationships: Vec<ChildRelationship>,
    pub compact_layoutable: bool,
    pub createable: bool,
    pub custom: bool,
    pub custom_setting: bool,
    pub deletable: bool,
    pub deprecated_and_hidden: bool,
    pub feed_enabled: bool,
    pub fields: Vec<Field>,
    pub has_subtypes: bool,
    pub is_subtype: bool,
    pub key_prefix: Option<String>,
    pub label: String,
    pub label_plural: String,
    pub layoutable: bool,
    pub listviewable: Option<bool>,
    pub lookup_layoutable: Option<bool>,
    pub mergeable: bool,
    pub mru_enabled: bool,
    pub name: String,
    pub queryable: bool,
    pub replicateable: bool,
    pub retrieveable: bool,
    pub search_layoutable: bool,
    pub searchable: bool,
    pub triggerable: bool,
    pub undeletable: bool,
    pub updateable: bool,
    pub urls: Urls,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub aggregatable: bool,
    pub ai_prediction_field: bool,
    pub auto_number: bool,
    pub byte_length: u32,
    pub calculated: bool,
    pub calculated_formula: Option<String>,
    pub cascade_delete: bool,
    pub case_sensitive: bool,
    pub compound_field_name: Option<String>,
    pub controller_name: Option<String>,
    pub createable: bool,
    pub custom: bool,
    pub default_value_formula: Option<String>,
    pub defaulted_on_create: bool,
    pub dependent_picklist: bool,
    pub deprecated_and_hidden: bool,
    pub digits: u8,
    pub display_location_in_decimal: bool,
    pub encrypted: bool,
    pub external_id: bool,
    pub extra_type_info: Option<String>,
    pub filterable: bool,
    pub filtered_lookup_info: Option<String>,
    pub formula_treat_null_number_as_zero: bool,
    pub groupable: bool,
    pub high_scale_number: bool,
    pub html_formatted: bool,
    pub id_lookup: bool,
    pub inline_help_text: Option<String>,
    pub label: String,
    pub length: u32,
    pub mask: Option<String>,
    pub mask_type: Option<String>,
    pub name: String,
    pub name_field: bool,
    pub name_pointing: bool,
    pub nillable: bool,
    pub permissionable: bool,
    pub polymorphic_foreign_key: bool,
    pub precision: u8,
    pub query_by_distance: bool,
    pub reference_target_field: Option<String>,
    pub relationship_name: Option<String>,
    pub relationship_order: Option<String>,
    pub restricted_delete: bool,
    pub restricted_picklist: bool,
    pub scale: u8,
    pub search_prefilterable: bool,
    pub soap_type: String,
    pub sortable: bool,
    #[serde(rename = "type")]
    pub field_type: String,
    pub unique: bool,
    pub updateable: bool,
    pub write_requires_master_read: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChildRelationship {
    pub cascade_delete: bool,
    #[serde(rename = "childSObject")]
    pub child_sobject: Option<String>,
    pub deprecated_and_hidden: bool,
    pub field: String,
    pub relationship_name: Option<String>,
    pub restricted_delete: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Urls {
    pub compact_layouts: String,
    pub row_template: String,
    pub approval_layouts: String,
    pub ui_detail_template: String,
    pub ui_edit_template: String,
    pub default_values: String,
    pub listviews: String,
    pub describe: String,
    pub ui_new_record: String,
    pub quick_actions: String,
    pub layouts: String,
    pub sobject: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DescribeGlobalResponse {
    pub encoding: String,
    pub max_batch_size: u16,
    pub sobjects: Vec<DescribeGlobalSObjectResponse>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DescribeGlobalSObjectResponse {
    pub activateable: bool,
    pub createable: bool,
    pub custom: bool,
    pub custom_setting: bool,
    pub deletable: bool,
    pub deprecated_and_hidden: bool,
    pub feed_enabled: bool,
    pub has_subtypes: bool,
    pub is_subtype: bool,
    pub key_prefix: Option<String>,
    pub label: String,
    pub label_plural: String,
    pub layoutable: bool,
    pub mergeable: bool,
    pub mru_enabled: bool,
    pub name: String,
    pub queryable: bool,
    pub replicateable: bool,
    pub retrieveable: bool,
    pub searchable: bool,
    pub triggerable: bool,
    pub undeletable: bool,
    pub updateable: bool,
    pub urls: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub search_records: Vec<SearchRecord>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchRecord {
    #[serde(rename = "Id")]
    pub id: String,
    pub attributes: SObjectAttribute,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SObjectAttribute {
    #[serde(rename = "type")]
    pub sobject_type: String,
    pub url: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionResponse {
    pub label: String,
    pub url: String,
    pub version: String,
}
