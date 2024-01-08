#version 460 core

in layout(location=0) vec3 position;
in layout(location=1) vec4 color;
in layout(location=2) vec3 normal;
uniform layout(location=0) mat4 mvp_matrix;
uniform layout(location=1) mat4 model_matrix;

out layout(location=0) vec4 vertexColor;
out layout(location=1) vec3 vertexNormal;

void main()
{
    // Color vector to pass to the fragment shader
    vertexColor = color;

    // Normal vector to pass to the fragment shader
    // vertexNormal = normal;
    vertexNormal = normalize(mat3(model_matrix) * normal);;

    // Transformed vertex
    gl_Position =  mvp_matrix * vec4(position, 1.0);

}
