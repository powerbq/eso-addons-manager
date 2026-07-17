#!/bin/bash

cd $(dirname $0)

for ts in translations/*.ts; do
    pyside6-lrelease "$ts" -qm "${ts%.ts}.qm"
done
