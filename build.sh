cd src/shader
glslc -fshader-stage=vert ./triangle.vert -o triangle_vert.spv
glslc -fshader-stage=frag ./triangle.frag -o triangle_frag.spv
