//! The major functions that deal with the database part of our dataset. Closely related to the 
//! *SealHits* project.

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
 *   db.rs - database access functions
 *   Author - bjb8@st-andrews.ac.uk
 * 
 *   Useful links:
 *   https://stackoverflow.com/questions/73559824/how-to-write-multiple-explicit-inner-joins-with-diesel
 */

use crate::models::Groups;
use crate::models::Images;
use crate::models::Points;
use crate::schema::groups;
use crate::schema::points;
use crate::schema::groups_images;
use crate::schema::images;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;


/// Establish a connection to a database using a url string
/// 
/// * `database_url` - a formatted string that describes our database connection.
pub fn establish_connection(database_url: String) -> PgConnection {
    dotenv().ok();

    // let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}


/// Return all the groups in the database
/// Requires a connection object. Returns a Vec of 'Groups'
/// 
/// * `conn` - a Diesel PgConnection object.
pub fn get_groups(conn: &mut diesel::pg::PgConnection) -> Vec<Groups> {
    let results = groups::table
        .select(Groups::as_select())
        .load(conn)
        .expect("Error loading groups");
    results
}


/// Return a group with this uid
/// Requires a connection object. Returns a 'Groups'
/// 
/// * `conn` - the Disel PgConnection object.
/// * `group_uid` - the uid for the group we want.
pub fn get_group(conn: &mut diesel::pg::PgConnection, group_uid: uuid::Uuid) -> Groups {
    let result = groups::table
        .select(Groups::as_select())
        .filter(groups::uid.eq(group_uid))
        .first(conn)
        .expect("Error loading groups");
    result
}


/// Return a group with this uid
/// Requires a connection object. Returns a 'Groups'
/// 
/// * `conn` - the Disel PgConnection object.
/// * `group_huid` - the huid for the group we want.
pub fn get_group_huid(conn: &mut diesel::pg::PgConnection, group_huid: String) -> Groups {
    let result = groups::table
        .select(Groups::as_select())
        .filter(groups::huid.eq(group_huid))
        .first(conn)
        .expect("Error loading groups");
    result
}

/// Return all the groups in the database up to a limit
/// 
/// Requires a connection object. Returns a Vec of 'Groups'
pub fn get_groups_limit(conn: &mut diesel::pg::PgConnection, limit: i64) -> Vec<Groups> {
    let results = groups::table
        .select(Groups::as_select())
        .limit(limit)
        .load(conn)
        .expect("Error loading groups");
    results
}

/// Return all Groups objects that match the sql field.
/// 
/// * `conn` - the Disel PgConnection object.
/// * `sql` - the sql string we want to match.
pub fn get_groups_sql(conn: &mut diesel::pg::PgConnection, sql: String) -> Vec<Groups> {
    diesel::sql_query(sql).load(conn).expect("Error in get_groups_sql")
}


/// Return all the groups in the database that match one of the codes passed in
/// 
/// * `conn` - the Disel PgConnection object.
/// * `codes` - a list of codes.
pub fn get_groups_classes(conn: &mut diesel::pg::PgConnection, codes: Vec<String>) -> Vec<Groups> {
    let results = groups::table
        .select(Groups::as_select())
        .filter(groups::code.eq_any(codes))
        .load(conn)
        .expect("Error loading groups");
    results
}

/// Return all the groups in the database that match one of the codes passed in up to a limit
/// 
/// * `conn` - the Disel PgConnection object.
/// * `codes` - a list of codes.
/// * `limit` - limit of how many to return.
pub fn get_groups_classes_limit(conn: &mut diesel::pg::PgConnection, codes: Vec<String>, limit: i64) -> Vec<Groups> {
    let results = groups::table
        .select(Groups::as_select())
        .filter(groups::code.eq_any(codes))
        .limit(limit)
        .load(conn)
        .expect("Error loading groups");
    results
}

/// Get all the Images for a particular group and sonar_id. Return in ascending time order
/// 
/// * `conn` - the Disel PgConnection object.
/// * `group_uuid` - the groups uid.
/// * `sonar_id` - the id of the sonar we want to return images for.
pub fn get_images_group(
    conn: &mut diesel::pg::PgConnection,
    group_uuid: uuid::Uuid,
    sonar_id: i32,
) -> Vec<Images> {
    let results: Vec<Images> = images::table
    .inner_join(groups_images::table.on(images::uid.eq(groups_images::image_id)))
    .inner_join(groups::table.on(groups::uid.eq(groups_images::group_id)))
    .filter(groups::uid.eq(group_uuid))
    .filter(images::sonarid.eq(sonar_id))
    .select(Images::as_select())
    .order(images::time)
    .load(conn)
    .expect("Error get_images_group");
    results
}

/// Return all the points for this group in this image. Could be zero
/// 
/// * `conn` - the Disel PgConnection object.
/// * `group_uuid` - the Groups object uid.
/// * `image_uuid` - the Images object uid.
pub fn get_points_group_image(
    conn: &mut diesel::pg::PgConnection,
    group_uuid: uuid::Uuid,
    image_uuid: uuid::Uuid
) -> Vec<Points> {
    let results: Vec<Points> = points::table
        .inner_join(groups::table.on(groups::uid.eq(points::group_id)))
        .inner_join(groups_images::table.on(groups_images::group_id.eq(groups::uid)))
        .inner_join(images::table.on(images::uid.eq(groups_images::image_id)))
        .filter(groups::uid.eq(group_uuid))
        .filter(images::uid.eq(image_uuid))
        .filter(points::sonarid.eq(images::sonarid))
        .filter(points::time.eq(images::time))
        .select(Points::as_select())
        .load(conn)
        .expect("Error points for image");

    results
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use uuid::uuid;
    use crate::bbs::points_to_bb;
    use super::*;
    use postgres::{Client, NoTls};
    use std::env;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::panic;

    
    // This closure and catch lets us catch failures but always fire off the teardown.
    // https://medium.com/@ericdreichert/test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab
    fn run_test<T>(test: T) -> ()
    where T: FnOnce() -> () + panic::UnwindSafe {
        setup();

        let result = panic::catch_unwind(|| {
            test()
        });

        teardown();
        assert!(result.is_ok())
    }


    fn setup() {
        // Load the test database into PostgreSQL just like with Sealhits
        let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
        d.push("testseals.sql");
        let sql_file_content = std::fs::read_to_string(d.as_path()).unwrap();
        let pg_user = env::var("SEALHITS_TESTDATA_PGUSER").unwrap();
        let pg_pass = env::var("SEALHITS_TESTDATA_PGPASS").unwrap();
        let pg_str: String = String::from_str("host=localhost user=").unwrap();
        let conn_string = pg_str + &pg_user + " password=" + &pg_pass;
        let mut client = Client::connect(conn_string.as_str(), NoTls).unwrap();
        client
            .batch_execute("CREATE USER testseals WITH PASSWORD 'testseals';")
            .unwrap();
        client.batch_execute("CREATE DATABASE testseals WITH OWNER testseals TEMPLATE = template0 ENCODING = 'UTF8' LOCALE_PROVIDER = libc LOCALE = 'C.UTF-8'").unwrap();
        client.close().unwrap();

        let conn_string2 = conn_string + " dbname=testseals";
        client = Client::connect(conn_string2.as_str(), NoTls).unwrap();
        client.batch_execute(&sql_file_content).unwrap();
        client.close().unwrap();
    }


    fn teardown() {
        // Remove the testseals database and user
        let pg_user = env::var("SEALHITS_TESTDATA_PGUSER").unwrap();
        let pg_pass = env::var("SEALHITS_TESTDATA_PGPASS").unwrap();
        let pg_str: String = String::from_str("host=localhost user=").unwrap();
        let conn_string = pg_str + &pg_user + " password=" + &pg_pass;
        let mut client = Client::connect(conn_string.as_str(), NoTls).unwrap();
        client.batch_execute("drop database testseals;").unwrap();
        client.batch_execute("drop user testseals;").unwrap();
    }

    
    #[test]
    #[serial]
    fn test_get_images_groups() {
        run_test(|| {
            let db_url = "postgres://testseals:testseals@localhost/testseals".to_string();
            let conn = &mut establish_connection(db_url);
            let sonar_id: i32 = 854;
            let group_uuid = uuid!("5854f637-0e84-4f2d-bbce-b8b902e94d50");
            let images = get_images_group(conn, group_uuid, sonar_id);
            // Should be 50 images for this group and this sonar
            assert_eq!(images.len(), 53);
        })
    }

    #[test]
    #[serial]
    fn test_groups_classes() {
        run_test(|| {
            let db_url = "postgres://testseals:testseals@localhost/testseals".to_string();
            let conn = &mut establish_connection(db_url);
            let codes : Vec<String> = vec![String::from("seal")];
            let groups = get_groups_classes(conn, codes);
            // Should be 6 images for this code
            assert_eq!(groups.len(), 6);

            let codes2 : Vec<String> = vec![String::from("fs")];
            let groups2 = get_groups_classes(conn, codes2);
            // Should be 0 groups for this code
            assert_eq!(groups2.len(), 0);

            let codes3 : Vec<String> = vec![String::from("db")];
            let groups3 = get_groups_classes(conn, codes3);
            // Should be 1 groups for this code
            assert_eq!(groups3.len(), 1);
        })
    }

    #[test]
    #[serial]
    fn test_get_points_group_image() {
        run_test(|| {
            let db_url = "postgres://testseals:testseals@localhost/testseals".to_string();
            let conn = &mut establish_connection(db_url);
            let group_uuid = uuid!("093e3fc2-338a-44fb-9993-066654507036");
            let image_uuid = uuid!("292f8c31-a41d-4256-9201-77c8effda338");
            let points = get_points_group_image(conn, group_uuid, image_uuid);
            let bb = points_to_bb(&points, 55.0);
            assert_eq!(points.len(), 1);
            assert!((bb.bearing_min - 0.775).abs() < 0.01);
            assert!((bb.bearing_max - 0.824).abs() < 0.01);
        })
    }
}