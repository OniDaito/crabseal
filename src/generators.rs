//! Generators start the pipelines, generating items that require processing.
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
 *   generators.rs - generators produce objects for our pipeline.
 *   Author - bjb8@st-andrews.ac.uk
 *   
 */
use crate::db::{
    establish_connection, get_groups, get_groups_limit, get_groups_sql, get_images_group,
    get_points_group_image,
};
use crate::image::{read_fits, ImageSize};
use crate::models::{Groups, Points};
use crate::ptypes::{GroupT, OriginT};
use diesel::PgConnection;
use image::{ImageBuffer, Luma};
use log::{info, warn};
use pbr::ProgressBar;
use rand::thread_rng;
use rand::seq::SliceRandom;
use scoped_threadpool::Pool;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};

// Now define some functions over these types. Generators create TypeObjects and nodes consume each TypeObjects
pub struct GeneratorGroups {
    groupts: Vec<GroupT>,
    start: usize,
    end: usize,
}


/// Generate a GroupT object - the start of our pipeline.
/// 
/// * `group` - the database Groups object we are starting with.
/// * `sonar_ids` - the sonar ids we are considering.
/// * `connection` - the Diesel PgConnection object.
/// * `min_window` - the minimum length of time permitted.
/// * `crop_height` - the height that all images are cropped to, regardless of source.
/// * `image_path_cache` - the image_path cache object.
/// * `code_to_id` - the mapping of codename to number.
fn gen_group(
    group: &Groups,
    sonar_ids: &Vec<i32>,
    connection: &mut PgConnection,
    min_window: u32,
    crop_height: u32,
    image_path_cache: &HashMap<String, PathBuf>,
    code_to_id: &HashMap<String, u8>,
) -> Option<GroupT> {
    let guid = group.uid;

    for sonar_id in sonar_ids {
        let mut track_len = 0;
        let mut track_start = 0;
        let mut track_end = 0;

        let images = get_images_group(connection, guid, *sonar_id);
        let mut pp: Vec<Vec<Points>> = vec![];

        for idx in 0..images.len() {
            let image = &images[idx];
            let points = get_points_group_image(connection, guid, image.uid);

            if points.len() > 0 {
                pp.push(points);
                track_len += 1;

                if track_start == -1 {
                    track_start = idx as i32;
                }

                if track_end < idx as i32 {
                    track_end = idx as i32;
                }
            } else {
                pp.push(vec![]); // Push a blank for this frame
            }
        }

        // Should always have some images but sometimes not for crazy dataset reasons.
        // Only accept ones with images and points. Could be a node but it's always critical so lets put it in here.
        // Track len needs to be extendable to 16 via interpolation potentially.

        if images.len() > 0 && track_end - track_start >= min_window as i32 && track_len > 3 {
            let image = &images[0];
            let image_path = image_path_cache.get(&image.filename).unwrap();
            let img_data: ImageBuffer<Luma<u8>, Vec<u8>> = read_fits(&image_path).unwrap();

            let img_size = ImageSize {
                width: img_data.width(), // All sonar are 512
                // Height varies between sonar but *occasionally* is a few pixels off even for the same sonar.
                // We *assume* not in this case. That 0 counts for all
                height: img_data.height(),
            };

            let crop_size = ImageSize {
                width: img_data.width(), // All sonar are 512
                height: crop_height,
            };

            let opt_classid = code_to_id.get(&group.code);

            if opt_classid.is_none() {
                warn!(
                    "Group {} on sonar {} has a class outside the ones selected in class_to_id.csv",
                    group.huid, sonar_id
                );
                return None;
            }
            let classid = *opt_classid.unwrap();

            let origin = OriginT {
                group: group.clone(),
                classid: classid,
                sonar_id: *sonar_id,
                img_size: img_size,
                crop_size: crop_size,
            };

            let ngt = GroupT {
                origin: origin,
                images: images.clone(),
                points: pp,
            };

            return Some(ngt);
        } else {
            // TODO - a bit too verbose
            //warn!("Group {} on sonar {} fails: imagelen {}, tracklen {}", group.huid, sonar_id, images.len(), track_len);
        }
    }
    None
}


impl GeneratorGroups {

    /// Create a new Generator that produces GroupT objects from our PostgreSQL database.
    /// 
    /// * `dbuser` - the database Groups object we are starting with.
    /// * `dbpass` - the sonar ids we are considering.
    /// * `dbname` - the Diesel PgConnection object.
    /// * `sonar_ids` - list of sonars to consider.
    /// * `image_path_cache` - the height that all images are cropped to, regardless of source.
    /// * `minimum_window` - the minimum time window in frames.
    /// * `dataset_limit` - possible limit on the number of GroupT objects to create.
    /// * `crop_height` - the height that all images are cropped to, regardless of source.
    /// * `sqlfilter` - path to the SQLFilter file.
    /// * `num_threads` - number of threads to use.
    /// * `code_to_id` - the mapping of codename to number.
    pub fn new(
        dbuser: &str,
        dbpass: &str,
        dbname: &str,
        sonar_ids: &Vec<i32>,
        image_path_cache: &HashMap<String, PathBuf>,
        minimum_window: usize,
        dataset_limit: usize,
        crop_height: u32,
        sqlfilter: &Option<PathBuf>,
        num_threads: u32,
        code_to_id: &HashMap<String, u8>,
    ) -> GeneratorGroups {
        // Generator groups connects to a database and creates it's own internal groupts
        // TODO - should be lazy when pulling from the DB - saves memory.
        let db_url = String::from("postgres://") + dbuser + ":" + dbpass + "@localhost/" + dbname;
        let thread_db_url = db_url.clone();
        let connection = &mut establish_connection(db_url);
        let groups: Vec<Groups>;
        let mut groupts: Vec<GroupT> = vec![];

        if dataset_limit > 0 {
            groups = get_groups_limit(connection, dataset_limit as i64);
        } else {
            if Option::is_some(sqlfilter) {
                let sqlquery: String =
                    fs::read_to_string(&mut sqlfilter.as_ref().unwrap()).unwrap(); // TODO - better error handling here!
                info!("Selecting groups via the SQLFilter file. {}", &sqlquery);
                groups = get_groups_sql(connection, sqlquery);
            } else {
                groups = get_groups(connection);
            }
        }

        // Track must be a minimum of two
        let mut min_window = minimum_window;
        if min_window < 2 {
            min_window = 2
        }

        // Generator is threaded - at least in it's initial creation, so we partition the groups
        // Now split off the groups into partitions as well as find their images.
        let (tx, rx) = channel::<Option<GroupT>>();
        let mut progress: i32 = 0;
        let mut pool = Pool::new(num_threads);
        let num_groups = groups.len();

        // Setup the threading parameters
        let mut gsplit: Vec<Vec<Groups>> = vec![];
        let mut gidx = 0;

        for _ in 0..num_threads {
            gsplit.push(vec![]);
        }

        info!("Partitioning groups...");
        let mut pb = ProgressBar::new(num_groups as u64);
        pb.format("╢▌▌░╟");

        for group in groups {
            gsplit[gidx].push(group.clone());
            gidx += 1;

            if gidx >= num_threads as usize {
                gidx = 0;
            }

            pb.inc();
        }

        info!("Processing groups...");
        let mut pb = ProgressBar::new(num_groups as u64);
        pb.format("╢▌▌░╟");

        // Start the threading
        pool.scoped(|scoped| {
            for _t in 0..num_threads {
                let cslice: &[Groups] = &gsplit[_t as usize];
                let tconn = thread_db_url.clone();
                let tx: Sender<_> = tx.clone();

                // Now execute the threads
                scoped.execute(move || {
                    // Perform the group processing
                    let thread_conn = &mut establish_connection(tconn);

                    for group in cslice {
                        let ogroup = gen_group(
                            group,
                            sonar_ids,
                            thread_conn,
                            min_window as u32,
                            crop_height,
                            image_path_cache,
                            code_to_id,
                        );
                        let _ = tx.send(ogroup);
                    }
                });
            }

            // Now receive on the channel and update the status bar
            // Update our progress bar
            while progress < num_groups as i32 {
                match rx.try_recv() {
                    Ok(ogroup) => {
                        pb.inc();
                        progress = progress + 1;

                        if ogroup.is_some() {
                            groupts.push(ogroup.unwrap());
                        }
                    }
                    Err(_e) => {}
                }
            }
        });

        let num_groups = groupts.len();
        info!("Final group size: {}", num_groups);

        GeneratorGroups {
            groupts: groupts,
            start: 0,
            end: num_groups,
        }
    }

    /// Return the number of GroupT objects this generator can create.
    pub fn size(&self) -> usize {
        self.groupts.len()
    }

    /// Shuffle the order of the GroupT objects.
    pub fn shuffle(&mut self) {
        self.groupts.shuffle(&mut thread_rng());
    }
}

impl Iterator for GeneratorGroups {
    type Item = GroupT;

    fn next(&mut self) -> Option<GroupT> {
        if self.start == self.end {
            None
        } else {
            let g = self.groupts[self.start].clone();
            let result = Some(g);
            self.start += 1;
            result
        }
    }
}

// *** TESTS ***
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
        path::Path,
        env,
        panic,
        str::FromStr,
    };
    use walkdir::WalkDir;
    use postgres::{Client, NoTls};
    use serial_test::serial;

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
    fn test_generators() {
        run_test(|| {
            let pg_user = env::var("SEALHITS_TESTDATA_PGUSER").unwrap();
            let pg_pass = env::var("SEALHITS_TESTDATA_PGPASS").unwrap();

            let sonar_ids = vec![853, 854];
            let mut d = PathBuf::from(env::var("SEALHITS_TESTDATA_DIR").unwrap());
            d.push("fits");
            let fits_path = d.as_os_str();
            let dataset_limit = 10;
            let mut img_paths: HashMap<String, PathBuf> = HashMap::new();
            let mut code_to_id: HashMap<String, u8> = HashMap::new();

            // Write to a cache file or read from it, if it already exits.
            let cache_path = "crabseal.cache";

            if Path::new(cache_path).exists() {
                let file = File::open(cache_path).unwrap();
                let reader = BufReader::new(file);

                for res in reader.lines() {
                    let line = res.unwrap().replace('\n', "");
                    let tokens = line.split(",").collect::<Vec<&str>>();
                    img_paths.insert(tokens[0].to_string(), Path::new(tokens[1]).to_path_buf());
                }
            } else {
                for file in WalkDir::new(fits_path)
                    .into_iter()
                    .filter_map(|file| file.ok())
                {
                    if file.metadata().unwrap().is_file() {
                        // This conversion to string from osstr is absolutely stupid!
                        let mut key = file.file_name().to_str().map(|s| s.to_string()).unwrap();
                        key = key.replace(".lz4", "");
                        img_paths.insert(key, file.path().to_path_buf());
                    }
                }
            }

            let code_class_path = "code_to_class.csv";

            if Path::new(code_class_path).exists() {
                let file = File::open(code_class_path).unwrap();
                let reader = BufReader::new(file);

                for res in reader.lines() {
                    let line = res.unwrap().replace('\n', "");
                    let tokens = line.split(",").collect::<Vec<&str>>();
                    code_to_id.insert(tokens[0].to_string(), tokens[1].parse::<u8>().unwrap());
                }
            }

            let mut generator = GeneratorGroups::new(
                pg_user.as_str(),
                pg_pass.as_str(),
                "testseals",
                &sonar_ids,
                &img_paths,
                4,
                dataset_limit,
                1632,
                &None,
                4,
                &code_to_id,
            );
            let _ = generator.next();
        });
    }
}
