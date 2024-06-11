//! The model of the database that reflects the one in the *SealHits* project.
//! We use Diesel to provide an ORM for us.
/** ```rust,ignore
 *     /\
 *    ( /   @ @    ()
 *     \  __| |__  /
 *      -/   "   \-
 *     /-|       |-\
 *    / /-\     /-\ \
 *     / /-`---'-\ \     
 *      /         \ CRABSEAL
 * 
 *   models.rs - db models for diesel
 *   Author - bjb8@st-andrews.ac.uk
 *   ```
 */

use uuid::Uuid;
use diesel::prelude::*;
use chrono::{DateTime, Utc};

// Order of fields should match these in the schema.rs

/// Each annotated Group is represented in the database as a Groups object.
#[derive(Queryable, Selectable, QueryableByName, Clone)]
#[diesel(table_name = crate::schema::groups)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Groups {
    pub gid: i64,
    pub timestart:DateTime<Utc>,
    pub interact: bool,
    pub mammal: i32,
    pub fish: i32,
    pub bird: i32,
    pub sqlite: String,
    pub uid: Uuid,
    pub code: String,
    pub comment: Option<String>,
    pub timeend: DateTime<Utc>,
    pub sqliteid: i64,
    pub split: i32,
    pub huid: String
}

/// An individual image from a sonar is represented as an Images object.
#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::images)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Images {
    pub filename: String,
    pub uid: Uuid,
    pub hastrack: bool,
    pub glf: String,
    pub time: DateTime<Utc>,
    pub sonarid: i32,
    pub range: f64
}


/// A PGDF file in use in the dataset.
#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::pgdfs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PGDFs {
    pub filename: String,
    pub startdate: DateTime<Utc>,
    pub enddate: DateTime<Utc>,
    pub uid: i64,    
}


/// Every point (min/max bearing and min/max distance) is stored as a Points object.
#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::points)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Points {
    pub time: DateTime<Utc>,
    pub sonarid: i32,
    pub minbearing: f32,
    pub maxbearing: f32,
    pub minrange: f32,
    pub maxrange: f32,
    pub track_id: Uuid,
    pub uid: Uuid,
    pub peakbearing: f32,
    pub peakrange: f32,
    pub maxvalue: f32,
    pub occupancy: f32,
    pub objsize: f32
}


/// This TracksGroups object links Tracks to Groups. 
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::tracks_groups)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TracksGroups {
   pub track_pam_id: i64,
   pub group_id: Uuid,
   pub binfile: String,
   pub track_id: Uuid
}