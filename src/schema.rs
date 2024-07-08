// @generated automatically by Diesel CLI.

diesel::table! {
    files (id) {
        id -> Text,
        site_id -> Text,
        name -> Text,
        path -> Text,
        mime_type -> Text,
        size -> BigInt,
        is_index -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    sites (id) {
        id -> Text,
        host -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        index_file -> Nullable<Text>,
    }
}

diesel::joinable!(files -> sites (site_id));

diesel::allow_tables_to_appear_in_same_query!(
    files,
    sites,
);
