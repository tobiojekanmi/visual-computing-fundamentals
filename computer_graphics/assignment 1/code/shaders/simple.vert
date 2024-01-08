#version 430 core

in vec3 position;

void main()
{
    // Define a 3x3 matrix
    mat3 flip_transform_mat = mat3(
        1.0, 0.0, 0.0,   // Row 1
        0.0, 1.0, 0.0,   // Row 2
        0.0, 0.0, 1.0    // Row 3
    );

    vec3 new_position = flip_transform_mat * position;
    gl_Position = vec4(new_position, 1.0);

}
