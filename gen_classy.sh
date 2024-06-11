#!/bin/bash
RED='\033[0;31m'
NC='\033[0m' # No Colour

echo -e "\U1F3CB " ${RED}GENERATING DATASET${NC} "\U1F3CB"

if ! git diff-files --quiet --ignore-submodules -- ; then
  echo "You have changes that have not been committed. Please commit before continuing."
  exit 1
fi

if (( $# < 6 ))
  then
    echo "Please pass the directory to save, the fits/input path, classes path, patchsize, sqlfilter and comments."
    exit 1
fi

baseops=""
date=$(date +'%Y_%m_%d')
echo $date > $basedir/notes.txt

gitbranch=$(git rev-parse --abbrev-ref HEAD)
basedir=$1
datadir=$2
classpath=$3
patchsize=$4
sqlfilter=$5
comment=$6
baseops="cargo run --release --bin gen_classy -- --outpath $basedir --fitspath $datadir --classpath $classpath --sqlfilter $sqlfilter -t 6"
echo "Running with default ops: " $baseops
echo $comment >> $basedir/notes.txt

git log --format=%B -n 1 HEAD >> $basedir/notes.txt

echo "Time to generate."

echo "# Version of code tag: " `git describe --abbrev=0 --tags` >> $basedir/notes.txt
echo "# branch " $gitbranch >> $basedir/notes.txt
echo "#git reset --hard " `git rev-parse HEAD` >> $basedir/notes.txt
echo "$baseops" >> $basedir/notes.txt

time $baseops

echo -e "\U1F37B"
