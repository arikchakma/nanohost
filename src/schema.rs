// @generated automatically by Diesel CLI.

diesel::table! {
    todos (id) {
        id -> Text,
        title -> Text,
        completed -> Bool,
        completed_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}
