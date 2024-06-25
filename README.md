# CrabSeal
CrabSeal focuses on generating a dataset from an already ingested set held inside a postgresql database and a directory full of FITS image files. If you have ingested data using the [SealHits](https://github.com/OniDaito/sealhits) project, you can create a dataset for [OceanMotion](https://github.com/OniDaito/oceanmotion) using this program.

Crabseal comprises two programs - *pipeline* and *pipeline_sector*. Both a very similar. The first creates images of the same dimensions as these passed in. The second version creates images that are smaller than these input - a map of sectors or segments. Depending on the version of *OceanMotion* you are running, you'll need one or the other.

CrabSeal creates sets of images - train, test and val. The default split is 80%, 16% and 4% for the train, test and validation sets. The images are placed in either *images/train*, *images/test* or *images/val*. 

## Pipelines
CrabSeal operates as a pipeline using functions referred to as *nodes*. Each node takes a particular input and outputs a changed object than can be passed along to another node.

The default pipeline for processing the data looks something like this:

1. Generate a raw track and it's corresponding stack of images.
2. Crop all images to a new fixed height (1632 by default, starting at 0,0 origin).
3. Interpolate the track to fill missing frames.
4. Make sure each frame in the track overlaps.
5. Perform a Kalman filter across the track.
6. Reject the track if there are significant deviation.
7. Run overlap a second time.
8. Convert the track to a pixel volume.
9. Optionally resize the track and data volumes.
10. Trim the volumes so there are no frames with no tracks.
11. Combine the track and data volumes as a paired datum.
12. Save the datum to numpy npz format, txt file and png.


## Building
Run the normal cargo commands:

    cargo build

This will pull in the dependencies and build both *pipeline* and *pipeline_sector*.

## Generating datasets

Firstly, regardless of which program you run, you'll need to create the output directory and place within it, two files. The first *code_to_class.csv* represents a mapping between the *code* field from the database and a number. A complete *code_to_class.csv* is included in this repository. Copy this to your output directory for your new dataset and remove any classes that won't appear.

The second file, *filter.sql* relates to the first in that it contains a single SQL line for filtering the data. The Groups allowed by this filter should all have values in their *code* field that contain a mapping in the *code_to_class.csv*. For example, if the *filter.sql* looks like this:

    SELECT * from groups where code = 'seal';

... then *code_to_class.csv* should contain a line such as:

    seal,1

... and no other as only groups with code 'seal' will be output by CrabSeal.

Your output directory should look like this:

    - <path for your dataset>
      |_ code_to_class.csv
      |_ filter.sql
      |_ readme.md

Always good to have a readme file so you know what this dataset is all about.

### pipeline
Assuming you have created the output directory and placed the *filter.sql* and *code_to_class.csv* into this directory, you can run:

    cargo run --release --bin pipeline -- -f ~/location/of/the/fits/images -o + ~/your/output/dir --width 256 --sqlfilter ~/your/output/dir/filter.sql --numframes 16

The numframes parameter refers to how long you want the data to be, in this case 16 frames.

### pipeline_sector
The same as pipeline, but the sectorsize parameter is important. This creates sectors at the full size, before the *width* transform, so in the case below, a sector size of 32 will appear as 16 as the width paramter is set to 256 - half of 512.

    cargo run --release --bin pipeline -- -f ~/location/of/the/fits/images -o + ~/your/output/dir --width 256 --sqlfilter ~/your/output/dir/filter.sql --numframes 16 --sectorsize 32

## Time-frames

Large datasets will take a while to generate. Groups selected from the database will be pre-processed. This takes fewer than 5 minutes for a set of 7000 or so items. The same number of items will take around 30 minutes to generate the final NPZ files during the second and final processing stage. Bigger sets of 10,000 items or so may take a couple of hours to generate.

## Testing
Testing requires the [sealhits_testdata zip file](https://zenodo.org/records/12518315). This is quite a large repository and requires git lfs to be installed. It includes images (as FITS files), GLFs, PGDFs and the schema & data for a postgresql test database. This database must be setup before testing can begin. Please refer to the README inthat particular project when setting up the database for testing. Once this is setup on your test machine, the pytest will create a temporary database called *testseals*. Make sure this database does not already exist.

You will need to export the username and password for your particular postgresql setup. This is done with a couple of environment variables

    export SEALHITS_TESTDATA_PGUSER="postgres"
    export SEALHITS_TESTDATA_PGpass="postgres"

This user needs have permissions to create and destroy databases and users.

It is likely this repository already exists if you've tested the project sealhits, therefore we use an environment variable that must be set before running the tests. On Linux, run the following in your shell:

    export SEALHITS_TESTDATA_DIR=<path to sealhits_testdata dir>

Replacing *<path to sealhits_testdata dir>* with the actual path.

Then run either:

    cargo test

or

    cargo test -- --nocapture

## Documentation
To build the documentation, execute the following command:

    cargo doc --open --no-deps --lib --bins

## Generating a schema file for Diesel
We include the schema file for the current *sealhits* database schema, but in case a new one needs making, the following instructions will generate a new schema file.

This program uses the Diesel ORM to access the postgresql database holding the seal data.

Diesel likes to look in .env when generating the schema:

    echo DATABASE_URL=postgres://sealhits:kissfromarose@localhost/sealhits > .env

The url is in the format:
    
    postgres://<username>:<password>@<host>/<database name>

Generate schemas with the following command

    diesel print-schema > src/schema.rs