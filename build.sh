#!/bin/bash

cd $(dirname $0)

./compile_translations.sh

mkdir -p build

cd build

pyinstaller --clean --distpath dist --workpath work ../app.spec
