#!/bin/bash
set -e
mkdir -p assets/images

mkdir -p assets/spvs
cd src/shader
for suffix in vert frag; do
	for file in *.$suffix; do
		glslc -fshader-stage=$suffix ./$file -o ../../assets/spvs/${file%.$suffix}_$suffix.spv
	done
done
