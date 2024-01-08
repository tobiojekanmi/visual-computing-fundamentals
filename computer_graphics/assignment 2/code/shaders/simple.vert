#version 430 core

in layout(location=0) vec3 position;
in layout(location=1) vec4 color;
uniform layout(location=0) float elapsed;
uniform layout(location=1) mat4 transforms_matrix;

out layout(location=0) vec4 vertexColor;

void main()
{
    // Color vector to pass to the fragment shader
    vertexColor = color;

    // Q3 - The affine transformation matrix 
    mat4 affine_transforms; 
    affine_transforms[0] = vec4(1, 0, 0, 0); // Column 1
    affine_transforms[1] = vec4(elapsed, 1, 0, 0); // Column 2
    affine_transforms[2] = vec4(0, 0, 1, 0); // Column 3
    affine_transforms[3] = vec4(0, 0, 0, 1); // Column 4

    // Transformed vertex
    vec4 new_position = transforms_matrix * vec4(position, 1.0);
    gl_Position =  new_position;

}
