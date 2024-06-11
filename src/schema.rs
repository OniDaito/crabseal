// @generated automatically by Diesel CLI.

diesel::table! {
    glfs (uid) {
        filename -> Varchar,
        startdate -> Timestamptz,
        enddate -> Timestamptz,
        uid -> Int8,
    }
}

diesel::table! {
    groups (uid) {
        gid -> Int8,
        timestart -> Timestamptz,
        interact -> Bool,
        mammal -> Int4,
        fish -> Int4,
        bird -> Int4,
        sqlite -> Varchar,
        uid -> Uuid,
        code -> Varchar,
        comment -> Nullable<Text>,
        timeend -> Timestamptz,
        sqliteid -> Int8,
        split -> Int4,
        huid -> Varchar,
    }
}

diesel::table! {
    groups_glfs (glf_id, group_id) {
        glf_id -> Int8,
        group_id -> Uuid,
    }
}

diesel::table! {
    groups_images (image_id, group_id) {
        image_id -> Uuid,
        group_id -> Uuid,
    }
}

diesel::table! {
    groups_pgdfs (pgdf_id, group_id) {
        pgdf_id -> Int8,
        group_id -> Uuid,
    }
}

diesel::table! {
    images (uid) {
        filename -> Varchar,
        uid -> Uuid,
        hastrack -> Bool,
        glf -> Varchar,
        time -> Timestamptz,
        sonarid -> Int4,
        range -> Float8,
    }
}

diesel::table! {
    pgdfs (uid) {
        filename -> Varchar,
        startdate -> Timestamptz,
        enddate -> Timestamptz,
        uid -> Int8,
    }
}

diesel::table! {
    points (uid) {
        time -> Timestamptz,
        sonarid -> Int4,
        minbearing -> Float4,
        maxbearing -> Float4,
        minrange -> Float4,
        maxrange -> Float4,
        peakbearing -> Float4,
        peakrange -> Float4,
        maxvalue -> Float4,
        occupancy -> Float4,
        objsize -> Float4,
        track_id -> Uuid,
        uid -> Uuid,
        group_id -> Uuid,
    }
}

diesel::table! {
    tracks_groups (track_id) {
        track_pam_id -> Int8,
        group_id -> Uuid,
        binfile -> Varchar,
        track_id -> Uuid,
    }
}

diesel::joinable!(groups_glfs -> glfs (glf_id));
diesel::joinable!(groups_glfs -> groups (group_id));
diesel::joinable!(groups_images -> groups (group_id));
diesel::joinable!(groups_images -> images (image_id));
diesel::joinable!(groups_pgdfs -> groups (group_id));
diesel::joinable!(groups_pgdfs -> pgdfs (pgdf_id));
diesel::joinable!(points -> groups (group_id));
diesel::joinable!(points -> tracks_groups (track_id));
diesel::joinable!(tracks_groups -> groups (group_id));

diesel::allow_tables_to_appear_in_same_query!(
    glfs,
    groups,
    groups_glfs,
    groups_images,
    groups_pgdfs,
    images,
    pgdfs,
    points,
    tracks_groups,
);
