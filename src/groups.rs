//! Diesel based functions for extracting the *SealHits* data from the PostgreSQL database - specifically for Groups objects.
/**
 *     /\
 *    ( /   @ @    ()
 *     \  __| |__  /
 *      -/   "   \-
 *     /-|       |-\
 *    / /-\     /-\ \
 *     / /-`---'-\ \     
 *      /         \ CRABSEAL
 *
 *   groups.rs - group selection routines
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */

use log::info;
use std::fs::read_to_string;
use std::path::PathBuf;
use crate::models::Groups;
use crate::db::{
    establish_connection, get_groups, get_groups_limit, get_groups_sql
};

/// Select the *best groups* from the database, performing any other pipeline bits for selection.
///
/// * `dbuser` - the database Groups object we are starting with.
/// * `dbpass` - the sonar ids we are considering.
/// * `dbname` - the Diesel PgConnection object.
/// * `sqlfilter` - Optional path to the SQLFilter file.
/// * `dataset_limit` - Optional limit - 0 means no limit.
pub fn select_groups(dbuser: &str, dbpass: &str, dbname: &str, sqlfilter: &Option<PathBuf>, dataset_limit: usize) -> Vec<Groups> {
    let db_url = String::from("postgres://")
        + dbuser
        + ":"
        + dbpass
        + "@localhost/"
        + dbname;
    let connection = &mut establish_connection(db_url);
    let groups: Vec<Groups>;

    // Initial filtering with an sqlfilter if it exists?
    if Option::is_some(sqlfilter) {
        let sqlquery: String = read_to_string(&mut sqlfilter.as_ref().unwrap()).unwrap(); // TODO - better error handling here!
        info!("Selecting groups via the SQLFilter file. {}", &sqlquery);
        groups = get_groups_sql(connection, sqlquery);
    } else {
        info!("Selecting all groups with limit: {}", dataset_limit);

        if dataset_limit > 0 {
            groups = get_groups_limit(connection, dataset_limit as i64);
        } else {
            groups = get_groups(connection);
        }
    }

    groups
}
