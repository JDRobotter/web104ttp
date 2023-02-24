// @generated automatically by Diesel CLI.

diesel::table! {
    blobs (id, side, thumbnail) {
        id -> Integer,
        side -> Integer,
        thumbnail -> Bool,
        data -> Binary,
        mime -> Text,
        show -> Bool,
        hash -> Integer,
    }
}

diesel::table! {
    pictures (id) {
        id -> Integer,
        word -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    blobs,
    pictures,
);
