#!/bin/bash
set -e
cd src/shader
for suffix in vert frag; do
	for file in *.$suffix; do
		glslc -fshader-stage=$suffix ./$file -o ${file%.$suffix}_suffix.spv
	done
done
